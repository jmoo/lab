{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeLinux;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      config = mkIf config.ulauncher.enable (
        homeLinux (
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

            # $launcher keybind referenced by hyprland.conf.
            wayland.windowManager.hyprland.settings."$launcher" =
              "${lib.getExe pkgs.uwsm} app -- ${lib.getExe ulauncher}";
          }
        )
      );

      options.ulauncher.enable = mkEnableOption "ulauncher application launcher";
    }
  );
}
