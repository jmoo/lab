{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeLinux;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.ghostty.enable = mkEnableOption "ghostty home-manager configuration";

      config = mkIf config.ghostty.enable (homeLinux {
        programs.ghostty = {
          enable = true;
          enableZshIntegration = true;
          settings = {
            background-opacity = 0.9;
            font-family = "UbuntuMono Nerd Font Mono";
            font-size = 13;
            gtk-tabs-location = "bottom";
            keybind = [
              "ctrl+shift+left=previous_tab"
              "ctrl+shift+right=next_tab"
              "ctrl+t=new_tab"
              "ctrl+shift+t=toggle_tab_overview"
              "ctrl+left=unbind"
              "ctrl+right=unbind"
            ];
            theme = "dark:Bright Lights,light:GruvboxLightHard";
            window-decoration = false;
          };
        };
      });
    }
  );
}
