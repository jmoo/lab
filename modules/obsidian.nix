{ lib', ... }:
let
  inherit (lib'.lab) homeLinux mkHostModule;
  inherit (lib')
    mkEnableOption
    mkIf
    mkMerge
    mkOption
    types
    ;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    let
      cfg = config.obsidian.sync;
    in
    {
      options.obsidian.sync = {
        enable = mkEnableOption "obsidian vault sync";

        interval = mkOption {
          default = "15m";
          description = "Systemd timer interval";
          type = types.str;
        };

        service = mkEnableOption "systemd timer for obsidian sync";

        vaults = mkOption {
          default = [ ];
          description = "Paths to obsidian vaults to sync";
          type = types.listOf types.str;
        };
      };

      config = mkIf cfg.enable (mkMerge [
        {
          home.module =
            { pkgs, ... }:
            let
              obsidian-sync = pkgs.writeShellApplication {
                name = "obsidian-sync";
                runtimeInputs = [ pkgs.git-sync ];
                text = lib'.concatMapStrings (v: "git-sync ${lib'.escapeShellArg v}\n") cfg.vaults;
              };
            in
            {
              home.packages = [ obsidian-sync ];
            };
        }

        (mkIf cfg.service (
          homeLinux (
            { config, ... }:
            {
              systemd.user = {
                services.obsidian-sync = {
                  Service = {
                    ExecStart = "${config.home.profileDirectory}/bin/obsidian-sync";
                    Type = "oneshot";
                  };
                  Unit.Description = "Obsidian vault sync";
                };

                timers.obsidian-sync = {
                  Install.WantedBy = [ "timers.target" ];
                  Timer = {
                    OnActiveSec = cfg.interval;
                    OnUnitActiveSec = cfg.interval;
                  };
                  Unit.Description = "Obsidian vault sync timer";
                };
              };
            }
          )
        ))
      ]);
    }
  );
}
