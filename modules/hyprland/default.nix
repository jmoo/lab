{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule;
  inherit (lib')
    mkEnableOption
    mkDefault
    mkIf
    ;

  # System (NixOS / Asahi) config to run a hyprland session via uwsm.
  system =
    { pkgs, ... }:
    {
      programs = {
        hyprland = {
          enable = true;
          withUWSM = true;
        };
        hyprlock.enable = true;
        xwayland.enable = true;
      };

      services.blueman.enable = true;

      environment.systemPackages = with pkgs; [
        kdePackages.qtwayland
        kdePackages.qtsvg
      ];
    };

  # Home config for the hyprland session. `nvidia` adds GPU session variables.
  home =
    nvidia:
    {
      pkgs,
      lib,
      config,
      ...
    }:
    let
      uwsm = cmd: "${lib.getExe pkgs.uwsm} app -- ${cmd}";

      terminal =
        if config.programs.ghostty.enable then
          lib.getExe config.programs.ghostty.package
        else
          lib.getExe pkgs.xterm;
    in
    {
      home.packages = with pkgs; [
        brightnessctl
        playerctl
        xterm
        nemo
        wdisplays
      ];

      services = {
        swaync.enable = true;
        flameshot.enable = true;
        network-manager-applet.enable = true;
      };

      systemd.user.services.network-manager-applet.Unit = {
        After = [ "graphical-session.target" ];
        PartOf = [ "graphical-session.target" ];
      };

      xdg = {
        mime.enable = true;
        portal.extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
      };

      wayland.windowManager.hyprland = {
        enable = true;
        # uwsm owns the systemd session, so disable home-manager's own unit.
        systemd.enable = false;
        xwayland.enable = true;
        extraConfig = builtins.readFile ./hyprland.conf;

        settings = {
          "$mod" = "SUPER";
          "$modShift" = "SUPER+SHIFT";
          "$modCtrl" = "SUPER+CTRL";
          "$modAlt" = "SUPER+ALT";
          "$modShiftCtrl" = "SUPER+SHIFT+CTRL";

          "$terminal" = uwsm terminal;

          env = [
            "QT_QPA_PLATFORM,wayland"
          ]
          ++ lib.optionals nvidia [
            "LIBVA_DRIVER_NAME,nvidia"
            "__GLX_VENDOR_LIBRARY_NAME,nvidia"
          ];
        };
      };
    };
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.hyprland = {
        enable = mkEnableOption "hyprland desktop";
        nvidia = mkEnableOption "set nvidia GPU session variables";
      };

      config = mkIf config.hyprland.enable {
        # Companion desktop modules, on by default with hyprland.
        greetd.enable = mkDefault true;
        theme.enable = mkDefault true;
        waybar.enable = mkDefault true;
        ulauncher.enable = mkDefault true;
        hyprlock.enable = mkDefault true;
        hyprpolkitagent.enable = mkDefault true;

        nixos = {
          module = system;
          home = home config.hyprland.nvidia;
        };
        asahi = {
          module = system;
          home = home config.hyprland.nvidia;
        };
      };
    }
  );
}
