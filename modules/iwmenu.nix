{
  config,
  lib,
  pkgs,
  ...
}:
with lib;
{
  options.lab.iwmenu = {
    enable = mkEnableOption "Enable iwmenu for wifi management";
    uwsm = mkEnableOption "Manages graphical-session systemd user targets and scopes with uwsm";
    package = mkOption {
      type = types.package;
      default = pkgs.iwmenu;
    };
  };

  config = mkIf config.lab.iwmenu.enable {
    home.packages = with pkgs; [
      iwmenu
      fuzzel
    ];

    programs.fuzzel.enable = true;

    lab.waybar.settings.network.on-click =
      "${optionalString config.lab.iwmenu.uwsm "uwsm app -- "}iwmenu -m fuzzel";
  };
}
