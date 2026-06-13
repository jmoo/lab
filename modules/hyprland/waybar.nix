{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeLinux;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      config = mkIf config.waybar.enable (
        homeLinux (
          { pkgs, lib, ... }:
          let
            uwsm = cmd: "${lib.getExe pkgs.uwsm} app -- ${cmd}";
          in
          {
            # Click targets for the bluetooth / audio modules.
            home.packages = with pkgs; [
              blueman
              pavucontrol
            ];

            programs.waybar = {
              enable = true;
              settings = [
                (lib.recursiveUpdate (builtins.fromJSON (builtins.readFile ./config.json)) {
                  bluetooth.on-click = uwsm "${pkgs.blueman}/bin/blueman-manager";
                  pulseaudio.on-click = uwsm (lib.getExe pkgs.pavucontrol);
                })
              ];
              style = builtins.readFile ./style.css;
            };

            systemd.user.services.waybar = {
              Install.WantedBy = [ "graphical-session.target" ];
              Service = {
                ExecStart = lib.getExe pkgs.waybar;
                Restart = "on-failure";
              };
              Unit = {
                After = [ "graphical-session.target" ];
                ConditionEnvironment = "WAYLAND_DISPLAY";
                Description = "waybar";
                PartOf = [ "graphical-session.target" ];
              };
            };
          }
        )
      );

      options.waybar.enable = mkEnableOption "waybar status bar";
    }
  );
}
