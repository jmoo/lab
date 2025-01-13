{
  config,
  lib,
  pkgs,
  ...
}:
with lib;
{
  options.lab.hyprpolkitagent = {
    enable = mkEnableOption "Use hyprpolkitagent as the default polkit password prompt service in home-manager";

    package = mkOption {
      type = types.package;
      default = pkgs.hyprpolkitagent;
    };
  };

  config = mkIf config.lab.hyprpolkitagent.enable {
    home.packages = [
      config.lab.hyprpolkitagent.package
    ];

    # Hyprpolkitagent already ships with a systemd service, we just need
    # to put it in the correct place so it will auto-start
    xdg.configFile."systemd/user/graphical-session.target.wants/hyprpolkitagent.service".source =
      "${config.lab.hyprpolkitagent.package}/share/systemd/user/hyprpolkitagent.service";
  };
}
