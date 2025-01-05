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
      # Utils
      btop
      blueberry
      hyprpolkitagent
      playerctl
      brightnessctl
      nemo
      pavucontrol
      wdisplays
      pop-launcher
      ulauncher-uwsm

      # Fonts
      nerd-fonts.ubuntu
      nerd-fonts.ubuntu-mono
      font-awesome

      # Theme
      adwaita-qt6
      adw-gtk3
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

    lab = {
      ulauncher.enable = true;
      waybar.enable = true;
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

    programs = {
      kitty.enable = true;
      ghostty.enable = true;

      hyprlock = {
        enable = true;
        extraConfig = builtins.readFile ./hyprlock.conf;
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

    wayland.windowManager.hyprland = {
      enable = true;
      xwayland.enable = true;
      systemd.enable = mkForce false;

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
