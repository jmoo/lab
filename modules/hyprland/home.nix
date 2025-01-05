{
  config,
  lib,
  pkgs,
  ...
}:
with lib;
{
  options.lab.hyprland = {
    enable = mkEnableOption "Enable hyprland home-manager configuration";
  };

  config = mkIf config.lab.hyprland.enable {
    home.packages = with pkgs; [
      nemo
      pavucontrol
      adwaita-qt6
      adw-gtk3
      nixos-artwork.wallpapers.dracula
      wdisplays
      pop-launcher
      ulauncher-uwsm
    ];

    dconf = {
      enable = true;
      settings = {
        "org/gnome/desktop/interface" = {
          color-scheme = "prefer-dark";
        };
      };
    };

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

    programs = {
      kitty.enable = true;
      ghostty.enable = true;

      hyprlock = {
        enable = true;
        extraConfig = builtins.readFile ./hyprlock.conf;
      };

      waybar = {
        enable = true;
        style = builtins.readFile ./waybar/style.css;
        settings = [
          (with builtins; fromJSON (readFile ./waybar/config.json))
        ];
      };
    };

    services = {
      hypridle = {
        enable = true;
      };

      network-manager-applet = {
        enable = true;
      };
    };

    systemd.user.services = {
      network-manager-applet.Unit.After = [ "graphical-session.target" ];
    };

    systemd.user.services.ulauncher = {
        Unit.Description = "Ulauncher - Application Runner";
        Install.WantedBy = ["graphical-session.target"];
        Unit.After = [ "graphical-session.target" ];
        Service = {
          ExecStart = "${pkgs.lib.getExe pkgs.ulauncher-uwsm}";
          Restart = "on-failure";
        };
      };

      systemd.user.services.waybar = {
        Unit.Description = "waybar";
        Install.WantedBy = ["graphical-session.target"];
        Unit.After = [ "graphical-session.target" ];
        Service = {
          ExecStart = "${pkgs.lib.getExe pkgs.waybar}";
          Restart = "on-failure";
        };
      };

    wayland.windowManager.hyprland = {
      enable = true;
      xwayland.enable = true;

      plugins = with pkgs.hyprlandPlugins; [
        hyprbars
      ];

      extraConfig = builtins.readFile ./hyprland.conf;

      settings = {
        "$mod" = "SUPER";
        "$modPrime" = "SUPER+SHIFT";
        "$terminal" = "uwsm app -- kitty";
        "$fileManager" = "uwsm app -- nemo";
        "$menu" = "uwsm app -- ulauncher-toggle";
        "$lock" = "uwsm app -- hyprlock";
        "$bar" = "uwsm app -- waybar";

        env = [
          "LIBVA_DRIVER_NAME,nvidia"
          "__GLX_VENDOR_LIBRARY_NAME,nvidia"
        ];
      };
    };
  };
}
