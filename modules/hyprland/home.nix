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
      font-awesome
      roboto
      ubuntu-sans-mono
      nixos-artwork.wallpapers.dracula
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

      hyprlock = {
        enable = true;
        extraConfig = builtins.readFile ./hyprlock.conf;
      };

      walker = {
        enable = true;
        runAsService = true;
        config = mkOptionDefault {
          "app_launch_prefix" = mkForce "uwsm app -- ";
        };
      };

      wlogout.enable = true;

      waybar = {
        enable = true;
        # systemd = {
        #   enable = true;
        #   target = "graphical-session.target";
        # };
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
    };

    wayland.windowManager.hyprland = {
      enable = true;
      xwayland.enable = true;

      extraConfig = builtins.readFile ./hyprland.conf;

      settings = {
        "$mod" = "SUPER";
        "$terminal" = "uwsm app -- kitty";
        "$fileManager" = "uwsm app -- nemo";
        "$menu" = "uwsm app -- walker -m applications";
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
