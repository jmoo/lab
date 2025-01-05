{
  lib,
  config,
  ...
}:
with lib;
{
  options.lab.waybar = {
    enable = mkEnableOption "Enable waybar";
  };

  config = mkIf config.lab.waybar.enable {
    programs = {
      waybar = {
        enable = true;
        style = builtins.readFile ./style.css;
        settings = [
          (with builtins; fromJSON (readFile ./config.json))
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
