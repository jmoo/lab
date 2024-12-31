{
  config,
  pkgs,
  lib,
  ...
}:
with lib;
{
  config = mkIf config.lab.hyprland.enable {
    environment.systemPackages = with pkgs; [
      btop
      blueberry
      hyprpolkitagent
      greetd.regreet
      playerctl
      brightnessctl
    ];

    home-manager.common = {
      lab.hyprland.enable = mkDefault true;
      wayland.windowManager.hyprland.systemd.enable = mkForce false;
    };

    programs = {
      hyprland = {
        enable = true;
        withUWSM = true;
      };

      hyprlock.enable = true;
      
      xwayland.enable = true;
    };

    services = {
      greetd = {
        enable = true;
        settings = {
          default_session = {
            command = "${pkgs.greetd.tuigreet}/bin/tuigreet --time --time-format '%I:%M %p | %a • %h | %F' --cmd \"uwsm start -S hyprland-uwsm.desktop\"";
            user = "greeter";
          };
        };
      };
    };
  };
}
