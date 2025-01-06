{
  lib,
  config,
  ...
}:
with lib;
{
  options.lab.waybar = {
    enable = mkEnableOption "Enable waybar home-manager configuration";

    settings = mkOption {
      type = with types; attrsOf anything;
      default = { };
    };
  };

  config = mkIf config.lab.waybar.enable {
    lab = {
      apps.bar.package = config.programs.waybar.package;
      waybar.settings = mkMerge [
        (with builtins; fromJSON (readFile ./config.json))
        {
          bluetooth.on-click = mkDefault config.lab.apps.bluetoothManager.command;
          pulseaudio.on-click = mkDefault config.lab.apps.audioManager.command;
        }
      ];
    };

    programs = {
      waybar = {
        enable = true;
        style = builtins.readFile ./style.css;
        settings = [
          config.lab.waybar.settings
        ];
      };
    };

    systemd.user.services.waybar = {
      Unit.Description = "waybar";
      Install.WantedBy = [ "graphical-session.target" ];
      Unit.After = [ "graphical-session.target" ];
      Service = {
        ExecStart = "${getExe config.programs.waybar.package}";
        Restart = "on-failure";
      };
    };
  };
}
