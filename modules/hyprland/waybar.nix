{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeLinux;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.waybar.enable = mkEnableOption "waybar status bar";

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
              style = builtins.readFile ./style.css;
              settings = [
                (lib.recursiveUpdate (builtins.fromJSON (builtins.readFile ./config.json)) {
                  bluetooth.on-click = uwsm "${pkgs.blueman}/bin/blueman-manager";
                  pulseaudio.on-click = uwsm (lib.getExe pkgs.pavucontrol);
                })
              ];
            };

            systemd.user.services.waybar = {
              Unit = {
                Description = "waybar";
                After = [ "graphical-session.target" ];
                PartOf = [ "graphical-session.target" ];
                ConditionEnvironment = "WAYLAND_DISPLAY";
              };
              Install.WantedBy = [ "graphical-session.target" ];
              Service = {
                ExecStart = lib.getExe pkgs.waybar;
                Restart = "on-failure";
              };
            };
          }
        )
      );
    }
  );
}
