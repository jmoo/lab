{
  config,
  lib,
  pkgs,
  wrapHyprCommand,
  ...
}:
with lib;
{
  options.lab.hypridle = {
    enable = mkEnableOption "Enable hypridle for inactivity timeout management in home-manager";

    lockCommand = mkOption {
      description = "Command that triggers the lock screen";
      type = types.str;
      default = wrapHyprCommand config.lab.apps.lock.command;
    };

    lockTimeout = mkOption {
      description = "Seconds until lock screen activates";
      type = with types; nullOr number;
      default = 900;
    };

    monitorTimeout = mkOption {
      description = "Seconds until monitors are turned off";
      type = with types; nullOr number;
      default = 1200;
    };
  };

  config = mkIf config.lab.hypridle.enable {
    home.packages = with pkgs; [
      nixos-artwork.wallpapers.dracula
    ];

    services.hypridle = {
      enable = true;

      settings = {
        general = {
          after_sleep_cmd = "hyprctl dispatch dpms on";
          ignore_dbus_inhibit = false;
          lock_cmd = config.lab.hypridle.lockCommand;
        };

        listener =
          (lists.optional (config.lab.hypridle.lockTimeout != null) {
            timeout = config.lab.hypridle.lockTimeout;
            on-timeout = config.lab.hypridle.lockCommand;
          })
          ++ (lists.optional (config.lab.hypridle.monitorTimeout != null) {
            timeout = config.lab.hypridle.monitorTimeout;
            on-timeout = "hyprctl dispatch dpms off";
            on-resume = "hyprctl dispatch dpms on";
          });
      };
    };
  };
}
