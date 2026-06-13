{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeLinux;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.hyprpolkitagent.enable = mkEnableOption "hyprpolkitagent polkit prompt";

      config = mkIf config.hyprpolkitagent.enable (
        homeLinux (
          { pkgs, ... }:
          {
            home.packages = [ pkgs.hyprpolkitagent ];

            # Ships its own user service; link it into the graphical session.
            xdg.configFile."systemd/user/graphical-session.target.wants/hyprpolkitagent.service".source =
              "${pkgs.hyprpolkitagent}/share/systemd/user/hyprpolkitagent.service";
          }
        )
      );
    }
  );
}
