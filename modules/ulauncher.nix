{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeLinux;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.ulauncher.enable = mkEnableOption "ulauncher application launcher";

      config = mkIf config.ulauncher.enable (
        homeLinux (
          { pkgs, lib, ... }:
          let
            # uwsm-aware build so launched apps land in the right systemd scope.
            ulauncher = pkgs.ulauncher-uwsm;
          in
          {
            home.packages = [ ulauncher ];

            # $launcher keybind referenced by hyprland.conf.
            wayland.windowManager.hyprland.settings."$launcher" =
              "${lib.getExe pkgs.uwsm} app -- ${lib.getExe ulauncher}";

            systemd.user.services.ulauncher = {
              Unit = {
                Description = "Ulauncher - Application Runner";
                After = [ "graphical-session.target" ];
                PartOf = [ "graphical-session.target" ];
                ConditionEnvironment = "WAYLAND_DISPLAY";
              };
              Install.WantedBy = [ "graphical-session.target" ];
              Service = {
                ExecStart = lib.getExe ulauncher;
                Restart = "on-failure";
              };
            };
          }
        )
      );
    }
  );
}
