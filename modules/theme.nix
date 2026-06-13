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
            dconf = {
              enable = true;
              settings."org/gnome/desktop/interface".color-scheme = "prefer-dark";
            };

            fonts.fontconfig.enable = true;

            gtk = {
              cursorTheme = {
                name = "Adwaita";
                package = pkgs.adwaita-icon-theme;
              };
              enable = true;
              gtk4.theme = config.gtk.theme;
              iconTheme = {
                name = "Adwaita";
                package = pkgs.adwaita-icon-theme;
              };
              theme.name = "adw-gtk3-dark";
            };

            home.packages = with pkgs; [
              adwaita-icon-theme
              adwaita-qt
              adwaita-qt6
              adw-gtk3

              nerd-fonts.ubuntu
              nerd-fonts.ubuntu-mono
              font-awesome
            ];

            qt = {
              enable = true;
              platformTheme.name = "adwaita";
              style.name = "adwaita-dark";
            };
          }
        )
      );
    }
  );
}
