{
  config,
  lib,
  ...
}:
with lib;
{
  options.lab.hyprlock = {
    enable = mkEnableOption "Use hyprlock as the default lock screen in home-manager";
  };

  config = mkIf config.lab.hyprlock.enable {
    lab.apps.lock.package = config.programs.hyprlock.package;

    programs.hyprlock = {
      enable = true;
      extraConfig = builtins.readFile ./config/hyprlock.conf;
    };
  };
}
