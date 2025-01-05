{
  pkgs,
  config,
  lib,
  ...
}:
with lib;
{
  options.lab.ulauncher = {
    enable = mkEnableOption "Enable ulauncher home-manager configuration";
    package = mkOption {
      type = types.package;
      default = pkgs.ulauncher-uwsm;
    };
  };

  config = mkIf config.lab.ulauncher.enable {
    systemd.user.services.ulauncher = {
      Unit.Description = "Ulauncher - Application Runner";
      Install.WantedBy = [ "graphical-session.target" ];
      Unit.After = [ "graphical-session.target" ];
      Service = {
        ExecStart = "${getExe config.lab.ulauncher.package}";
        Restart = "on-failure";
      };
    };
  };
}
