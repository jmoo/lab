{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeLinux hyprVar;
  inherit (lib') mkEnableOption mkIf mkMerge;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      config = mkIf config.ulauncher.enable (mkMerge [
        (homeLinux (
          { pkgs, lib, ... }:
          let
            # uwsm-aware build so launched apps land in the right systemd scope.
            ulauncher = pkgs.ulauncher-uwsm;
          in
          {
            home.packages = [ ulauncher ];

            systemd.user.services.ulauncher = {
              Install.WantedBy = [ "graphical-session.target" ];
              Service = {
                ExecStart = lib.getExe ulauncher;
                Restart = "on-failure";
              };
              Unit = {
                After = [ "graphical-session.target" ];
                ConditionEnvironment = "WAYLAND_DISPLAY";
                Description = "Ulauncher - Application Runner";
                PartOf = [ "graphical-session.target" ];
              };
            };
          }
        ))

        # `launcher` keybind in the hyprland config.
        (hyprVar "launcher" (
          { pkgs, lib, ... }: "${lib.getExe pkgs.uwsm} app -- ${lib.getExe pkgs.ulauncher-uwsm}"
        ))
      ]);

      options.ulauncher.enable = mkEnableOption "ulauncher application launcher";
    }
  );
}
