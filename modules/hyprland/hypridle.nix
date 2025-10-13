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

    suspendCommand = mkOption {
      description = "Command that triggers the systemd suspend/sleep";
      type = types.str;
      default = wrapHyprCommand config.lab.apps.suspend.command;
    };

    displayCommand = {
      on = mkOption {
        description = "Command that enables display";
        type = types.str;
        default = wrapHyprCommand config.lab.apps.displayOn.command;
      };

      off = mkOption {
        description = "Command that enables display";
        type = types.str;
        default = wrapHyprCommand config.lab.apps.displayOff.command;
      };
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

    suspendTimeout = mkOption {
      description = "Seconds until device goes to sleep";
      type = with types; nullOr number;
      default = 1201;
    };
  };

  config = mkIf config.lab.hypridle.enable {
    home.packages = with pkgs; [
      nixos-artwork.wallpapers.dracula
    ];

    lab.apps = {
      displayOn = mkDefault {
        command = "hyprctl dispatch dpms on";
      };

      displayOff = mkDefault {
        command = "hyprctl dispatch dpms off";
      };

      suspend = mkDefault {
        command = "systemctl suspend";
      };
    };

    services = {
      hypridle = {
        enable = true;

        settings = {
          general = {
            after_sleep_cmd = config.lab.hypridle.displayCommand.on;
            before_sleep_cmd = config.lab.hypridle.lockCommand;
            ignore_dbus_inhibit = false;
            lock_cmd = config.lab.hypridle.lockCommand;
            inhibit_sleep = 3;
          };

          listener =
            (lists.optional (config.lab.hypridle.lockTimeout != null) {
              timeout = config.lab.hypridle.lockTimeout;
              on-timeout = config.lab.hypridle.lockCommand;
            })
            ++ (lists.optional (config.lab.hypridle.monitorTimeout != null) {
              timeout = config.lab.hypridle.monitorTimeout;
              on-timeout = config.lab.hypridle.displayCommand.on;
              on-resume = config.lab.hypridle.displayCommand.off;
            })
            ++ (lists.optional (config.lab.hypridle.suspendTimeout != null) {
              timeout = config.lab.hypridle.suspendTimeout;
              on-timeout = config.lab.hypridle.suspendCommand;
            });
        };
      };
    };
  };
}
