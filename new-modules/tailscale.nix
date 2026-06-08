{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule forLinux;
  inherit (lib')
    mkEnableOption
    mkIf
    ;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    let
      cfg = config.tailscale;
    in
    {
      options.tailscale = {
        enable = mkEnableOption "tailscale nixos configuration";

        exitNode = mkEnableOption "advertise this host as an exit node / subnet router";
      };

      config = mkIf cfg.enable (
        forLinux (
          {
            config,
            pkgs,
            lib,
            ...
          }:
          let
            tailscale = config.services.tailscale.package;
            exitNode = cfg.exitNode;
          in
          {
            services.tailscale = {
              enable = true;
              useRoutingFeatures = if exitNode then "server" else "client";
            };

            environment.systemPackages = [
              config.services.tailscale.package
            ];

            networking.firewall.checkReversePath = lib.mkIf exitNode "loose";

            systemd.services.tailscale-autoconnect = {
              description = "Automatic connection to Tailscale";
              serviceConfig.Type = "oneshot";

              script = with pkgs; ''
                # wait for tailscaled to settle
                sleep 10

                # check if we are already authenticated to tailscale
                status="$(${tailscale}/bin/tailscale status -json | ${jq}/bin/jq -r .BackendState)"
                if [[ "$status" == "Running" ]]; then # if so, then do nothing
                  exit 0
                fi

                # otherwise authenticate with tailscale
                ${tailscale}/bin/tailscale up -authkey $(cat /etc/tailscale/key) ${lib.optionalString exitNode "--advertise-exit-node"} \
                  --snat-subnet-routes=false \
                  --advertise-routes=10.10.0.0/16
              '';

              after = [
                "network-pre.target"
                "tailscale.service"
              ];
              wants = [
                "network-pre.target"
                "tailscale.service"
              ];
              wantedBy = [ "multi-user.target" ];
            };
          }
        )
      );
    }
  );
}
