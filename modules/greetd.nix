{
  pkgs,
  lib,
  config,
  ...
}:
with lib;
{
  options.lab.greetd = {
    enable = mkEnableOption "Enable greetd nixos configuration";
  };

  config = mkIf config.lab.greetd.enable {
    services = {
      greetd = {
        enable = true;
        settings = {
          default_session = {
            command = "${pkgs.greetd.tuigreet}/bin/tuigreet --time --time-format '%I:%M %p | %a • %h | %F' #--cmd \"uwsm start -S hyprland-uwsm.desktop\"";
            user = "greeter";
          };
        };
      };
    };
  };
}
