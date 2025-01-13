{
  config,
  lib,
  ...
}:
with lib;
{
  options.lab.hyprpaper = {
    enable = mkEnableOption "Enable hyprpaper for wallpaper management in home-manager";
  };

  config = mkIf config.lab.hyprpaper.enable {

    services.hyprpaper = {
      enable = true;

      settings = {
        ipc = "on";
        splash = false;
        # preload = config.lab.hyprpaper.paper;
        # wallpapers = config.lab.hyprpaper.paper;
      };
    };
  };
}
