# Config for the guake terminal for gnome
{ config, pkgs, lib, ... }:

with lib;
with builtins;

let mkTuple = lib.hm.gvariant.mkTuple;
in {
  options.lab.guake.enable = mkEnableOption "guake";

  config.home = mkIf config.lab.guake.enable {
    packages = with pkgs; [ guake ];

    file.guake-desktop = {
      executable = false;
      source = "${pkgs.guake}/share/applications/guake.desktop";
      target = ".local/share/applications/guake.desktop";
    };

    file.guake-prefs-desktop = {
      executable = false;
      source = "${pkgs.guake}/share/applications/guake-prefs.desktop";
      target = ".local/share/applications/guake-prefs.desktop";
    };
  };

  config.dconf.settings = mkIf config.lab.guake.enable {
    "apps/guake/general" = {
      compat-delete = "delete-sequence";
      display-n = 0;
      display-tab-names = 0;
      gtk-use-system-default-theme = true;
      hide-tabs-if-one-tab = false;
      history-size = 1000;
      infinite-history = true;
      load-guake-yml = true;
      max-tab-name-length = 100;
      mouse-display = true;
      new-tab-after = true;
      open-tab-cwd = true;
      prompt-on-quit = true;
      quick-open-command-line = "gedit %(file_path)s";
      restore-tabs-notify = true;
      restore-tabs-startup = true;
      save-tabs-when-changed = true;
      schema-version = "3.9.0";
      scroll-keystroke = true;
      set-window-title = true;
      start-at-login = true;
      use-default-font = false;
      use-login-shell = true;
      use-popup = true;
      use-scrollbar = true;
      use-trayicon = true;
      window-halignment = 0;
      window-height = 63;
      window-losefocus = false;
      window-refocus = true;
      window-tabbar = true;
      window-valignment = 1;
      window-vertical-displacement = 310;
      window-width = 33;
    };

    "apps/guake/keybindings/global" = { show-hide = "<Super>c"; };

    "apps/guake/keybindings/local" = {
      new-tab = "<Primary>t";
      next-tab-alt = "<Alt>Right";
      previous-tab-alt = "<Alt>Left";
    };

    "apps/guake/style/background" = { transparency = 94; };

    "apps/guake/style/font" = {
      allow-bold = true;
      palette =
        "#000000000000:#cccc00000000:#4e4e9a9a0606:#c4c4a0a00000:#34346565a4a4:#757550507b7b:#060698209a9a:#d3d3d7d7cfcf:#555557575353:#efef29292929:#8a8ae2e23434:#fcfce9e94f4f:#72729f9fcfcf:#adad7f7fa8a8:#3434e2e2e2e2:#eeeeeeeeecec:#ffffffffffff:#13331614170a";
      palette-name = "Custom";
      style = "MesloLGS NF 10";
    };
  };
}
