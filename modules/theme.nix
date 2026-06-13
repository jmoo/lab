{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeLinux;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.theme.enable = mkEnableOption "dark gtk/qt theme + fonts";

      config = mkIf config.theme.enable (
        homeLinux (
          { pkgs, config, ... }:
          {
            home.packages = with pkgs; [
              adwaita-icon-theme
              adwaita-qt
              adwaita-qt6
              adw-gtk3

              nerd-fonts.ubuntu
              nerd-fonts.ubuntu-mono
              font-awesome
            ];

            fonts.fontconfig.enable = true;

            dconf = {
              enable = true;
              settings."org/gnome/desktop/interface".color-scheme = "prefer-dark";
            };

            gtk = {
              enable = true;
              theme.name = "adw-gtk3-dark";
              iconTheme = {
                name = "Adwaita";
                package = pkgs.adwaita-icon-theme;
              };
              cursorTheme = {
                name = "Adwaita";
                package = pkgs.adwaita-icon-theme;
              };
              gtk4.theme = config.gtk.theme;
            };

            qt = {
              enable = true;
              style.name = "adwaita-dark";
              platformTheme.name = "adwaita";
            };
          }
        )
      );
    }
  );
}
