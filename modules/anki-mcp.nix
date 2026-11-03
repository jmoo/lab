{ lib', ... }:
let
  inherit (lib'.lab) forLinux mkHostModule;
  inherit (lib')
    mkEnableOption
    mkIf
    mkOption
    types
    ;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    let
      cfg = config.anki-mcp;
      user = config.user;
    in
    {
      options.anki-mcp = {
        enable = mkEnableOption "private read-only Anki MCP access with managed Anki";
        profile = mkOption {
          default = "User 1";
          description = "Provisioned Anki profile under /var/lib/anki-headless";
          type = types.str;
        };
        syncCalendar = mkOption {
          default = "*:0/5";
          description = "systemd calendar expression for AnkiWeb syncs; null disables automatic syncing";
          type = types.nullOr types.str;
        };
        tunnelApiKeyFile = mkOption {
          default = "/etc/anki-mcp/api-key";
          description = "Runtime file containing an OpenAI key with Tunnels Read + Use";
          type = types.str;
        };
        tunnelEnvironmentFile = mkOption {
          default = "/etc/anki-mcp/tunnel.env";
          description = "Runtime systemd environment file containing CONTROL_PLANE_TUNNEL_ID";
          type = types.str;
        };
      };

      config = mkIf cfg.enable (
        forLinux (
          {
            config,
            pkgs,
            utils,
            ...
          }:
          let
            anki = pkgs.anki.withAddons [
              (pkgs.ankiAddons.anki-connect.withConfig {
                config = {
                  apiKey = null;
                  apiLogPath = null;
                  webBindAddress = "127.0.0.1";
                  webBindPort = 8765;
                  webCorsOriginList = [ ];
                };
              })
            ];
            dataDirectory = "/var/lib/anki-headless";
            provisioned = "${dataDirectory}/.provisioned";
            waitForAnki = pkgs.writeShellScript "anki-wait" ''
              for attempt in {1..60}; do
                if ${pkgs.curl}/bin/curl --fail --silent --max-time 2 \
                  -H 'Content-Type: application/json' \
                  -d '{"action":"deckNames","version":6}' http://127.0.0.1:8765 \
                  | ${pkgs.jq}/bin/jq -e '.error == null and (.result | type == "array")' >/dev/null; then
                  exit 0
                fi
                sleep 1
              done
              echo 'AnkiConnect did not become ready; check the provisioned profile' >&2
              exit 1
            '';
          in
          {
            environment.systemPackages = [
              anki
              pkgs.anki-tool
            ];

            systemd = {
              services = {
                anki-headless = {
                  after = [ "network-online.target" ];
                  description = "Managed Anki profile with loopback-only AnkiConnect";
                  environment = {
                    ANKI_WAYLAND = "0";
                    LIBGL_ALWAYS_SOFTWARE = "1";
                    QT_QPA_PLATFORM = "xcb";
                    QTWEBENGINE_CHROMIUM_FLAGS = "--disable-gpu";
                  };
                  # ⚠️ Another Anki on 8765 would pass ExecStartPost against the wrong profile.
                  preStart = ''
                    if [ -n "$(${pkgs.iproute2}/bin/ss -Hltn 'sport = :8765')" ]; then
                      echo 'Port 8765 is in use; close the other Anki instance' >&2
                      exit 1
                    fi
                  '';
                  serviceConfig = {
                    ExecStart = utils.escapeSystemdExecArgs [
                      "${pkgs.xvfb-run}/bin/xvfb-run"
                      "-a"
                      "-s"
                      "-screen 0 1280x800x24 -nolisten tcp"
                      "${anki}/bin/anki"
                      "-b"
                      dataDirectory
                      "-p"
                      cfg.profile
                    ];
                    ExecStartPost = waitForAnki;
                    PrivateTmp = true;
                    Restart = "on-failure";
                    RestartSec = 10;
                    StateDirectory = "anki-headless";
                    StateDirectoryMode = "0700";
                    TimeoutStartSec = 200;
                    User = user;
                    WorkingDirectory = dataDirectory;
                  };
                  unitConfig.ConditionPathExists = provisioned;
                  wantedBy = [ "multi-user.target" ];
                  wants = [ "network-online.target" ];
                };

                anki-headless-sync = mkIf (cfg.syncCalendar != null) {
                  after = [ "anki-headless.service" ];
                  description = "Sync the managed Anki profile with AnkiWeb";
                  requires = [ "anki-headless.service" ];
                  serviceConfig = {
                    ExecStart = "${pkgs.anki-tool}/bin/anki-tool sync";
                    StandardOutput = "null";
                    TimeoutStartSec = 90;
                    Type = "oneshot";
                    User = user;
                  };
                  unitConfig.ConditionPathExists = provisioned;
                };

                anki-mcp-tunnel = {
                  after = [
                    "anki-headless.service"
                    "network-online.target"
                  ];
                  description = "Private Anki MCP tunnel to OpenAI";
                  environment.HOME = "/var/lib/anki-mcp-tunnel";
                  partOf = [ "anki-headless.service" ];
                  requires = [ "anki-headless.service" ];
                  serviceConfig = {
                    DynamicUser = true;
                    EnvironmentFile = cfg.tunnelEnvironmentFile;
                    ExecStart = "${pkgs.tunnel-client}/bin/tunnel-client run --control-plane.api-key=file:%d/api-key --health.listen-addr=127.0.0.1:8766 --mcp.stdio-failure-policy=exit --mcp.command='${pkgs.anki-mcp}/bin/anki-mcp --anki-tool ${pkgs.anki-tool}/bin/anki-tool'";
                    LoadCredential = "api-key:${cfg.tunnelApiKeyFile}";
                    NoNewPrivileges = true;
                    PrivateTmp = true;
                    ProtectHome = true;
                    ProtectSystem = "strict";
                    Restart = "on-failure";
                    RestartSec = 10;
                    RestrictAddressFamilies = [
                      "AF_INET"
                      "AF_INET6"
                      "AF_UNIX"
                    ];
                    StateDirectory = "anki-mcp-tunnel";
                    StateDirectoryMode = "0700";
                    WorkingDirectory = "/var/lib/anki-mcp-tunnel";
                  };
                  unitConfig.ConditionPathExists = [
                    provisioned
                    cfg.tunnelApiKeyFile
                    cfg.tunnelEnvironmentFile
                  ];
                  wantedBy = [ "multi-user.target" ];
                  wants = [ "network-online.target" ];
                };
              };

              timers.anki-headless-sync = mkIf (cfg.syncCalendar != null) {
                description = "Periodically sync the managed Anki profile";
                timerConfig.OnCalendar = cfg.syncCalendar;
                wantedBy = [ "timers.target" ];
              };

              # ⚠️ Unit conditions are checked before StateDirectory is created.
              tmpfiles.rules = [
                "d /etc/anki-mcp 0700 root root - -"
                "d ${dataDirectory} 0700 ${user} ${config.users.users.${user}.group} - -"
              ];
            };
          }
        )
      );
    }
  );
}
