{
  pkgs,
  lib,
  config,
  ...
}:
with lib;
{
  options.lab.theme.enable = mkEnableOption "Enable theme";

  config = mkIf config.lab.theme.enable {
    home.packages = with pkgs; [
      # Theme
      adwaita-icon-theme
      adwaita-qt
      adwaita-qt6
      adw-gtk3

      # Fonts
      nerd-fonts.ubuntu
      nerd-fonts.ubuntu-mono
      font-awesome
    ];

    dconf = {
      enable = true;
      settings = {
        "org/gnome/desktop/interface" = {
          color-scheme = "prefer-dark";
        };
      };
    };

    fonts.fontconfig.enable = true;

    gtk = {
      enable = true;

      theme = {
        name = "adw-gtk3-dark";
      };

      iconTheme = {
        name = "Adwaita";
        package = pkgs.adwaita-icon-theme;
      };

      cursorTheme = {
        name = "Adwaita";
        package = pkgs.adwaita-icon-theme;
      };
    };

    qt = {
      enable = true;
      style.name = "adwaita-dark";
      platformTheme.name = "adwaita";
    };
  };
}
