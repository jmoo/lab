{
  config,
  lib,
  ...
}:
with lib;
{
  options.lab.ghostty = {
    enable = mkEnableOption "Enable ghostty home-manager configuration";
  };

  config = mkIf config.lab.ulauncher.enable {
    lab.apps.terminal.package = config.programs.ghostty.package;

    programs.ghostty = {
      enable = true;

      settings = {
        theme = "dark:Bright Lights,light:GruvboxLightHard";
        background-opacity = 0.9;
        font-family = "UbuntuMono Nerd Font Mono";
        font-size = 14;
        window-decoration = false;
        gtk-tabs-location = "bottom";
        keybind = [
          "ctrl+left=previous_tab"
          "ctrl+right=next_tab"
          "ctrl+t=new_tab"
          # "ctrl+shift+t=toggle_tab_overview"
          # "ctrl+plus=increase_font_size"
          # "ctrl+minus=decrease_font_size"
        ];
      };
    };
  };
}
