# My cinnamon desktop configuration for linux mint machines
{ config, pkgs, lib, ... }:

with lib;
with builtins;

let mkTuple = lib.hm.gvariant.mkTuple;
in {
  options.lab.cinnamon.enable = mkEnableOption "cinnamon";

  config.dconf.settings = mkIf config.lab.cinnamon.enable {
    "org/cinnamon/desktop/keybindings" = {
      custom-list = [ "custom3" "custom2" "custom1" "__dummy__" "custom0" ];
      looking-glass-keybinding = [ ];
    };

    "org/cinnamon/desktop/keybindings/custom-keybindings/custom0" = {
      binding = [ "<Super>Left" ];
      command = "xdotool mousemove 500 0 click 1";
      name = "focus left window";
    };

    "org/cinnamon/desktop/keybindings/custom-keybindings/custom1" = {
      binding = [ "<Super>Right" ];
      command = "xdotool mousemove 5000 0 click 1";
      name = "focus right window";
    };

    "org/cinnamon/desktop/keybindings/custom-keybindings/custom2" = {
      binding = [ "<Super>Home" ];
      command = "xdotool mousemove 2500 0 click 1";
      name = "focus center window";
    };

    "org/cinnamon/desktop/keybindings/custom-keybindings/custom3" = {
      binding = [ "<Super>c" ];
      command = "guake-toggle";
      name = "toggle guake";
    };

    "org/cinnamon/desktop/keybindings/media-keys" = {
      screensaver = [ "<Super>l" "XF86ScreenSaver" ];
      terminal = [ "<Primary><Alt>t" ];
    };

    "org/cinnamon/desktop/keybindings/wm" = {
      move-to-monitor-down = [ ];
      move-to-monitor-up = [ ];
      move-to-workspace-left = [ "<Primary><Super>Up" ];
      move-to-workspace-right = [ "<Primary><Super>Down" ];
      push-tile-down = [ ];
      push-tile-left = [ ];
      push-tile-right = [ ];
      push-tile-up = [ ];
      switch-to-workspace-left = [ "<Super>Up" ];
      switch-to-workspace-right = [ "<Super>Down" ];
    };

    "org/gtk/settings/file-chooser" = {
      date-format = "regular";
      location-mode = "path-bar";
      show-hidden = true;
      show-size-column = true;
      show-type-column = true;
      sidebar-width = 150;
      sort-column = "modified";
      sort-directories-first = true;
      sort-order = "descending";
      type-format = "category";
      window-position = mkTuple [ 2042 383 ];
      window-size = mkTuple [ 1096 822 ];
    };

    "org/nemo/desktop" = {
      computer-icon-visible = false;
      home-icon-visible = false;
      volumes-visible = false;
    };

    "org/nemo/preferences" = {
      show-hidden-files = true;
      show-location-entry = true;
    };

    "org/nemo/window-state" = {
      geometry = "1098x694+1922+356";
      maximized = false;
      sidebar-bookmark-breakpoint = 5;
      start-with-sidebar = true;
    };
  };
}
