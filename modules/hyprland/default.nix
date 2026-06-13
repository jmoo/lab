{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule;
  inherit (lib')
    mkDefault
    mkEnableOption
    mkIf
    ;

  # System (NixOS / Asahi) config to run a hyprland session via uwsm.
  system =
    { pkgs, ... }:
    {
      environment.systemPackages = with pkgs; [
        kdePackages.qtwayland
        kdePackages.qtsvg
      ];

      programs = {
        hyprland = {
          enable = true;
          withUWSM = true;
        };
        hyprlock.enable = true;
        xwayland.enable = true;
      };

      services.blueman.enable = true;
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
        flameshot.enable = true;
        network-manager-applet.enable = true;
        swaync.enable = true;
      };

      systemd.user.services.network-manager-applet.Unit = {
        After = [ "graphical-session.target" ];
        PartOf = [ "graphical-session.target" ];
      };

      wayland.windowManager.hyprland = {
        enable = true;
        extraConfig = builtins.readFile ./hyprland.conf;

        settings = {
          "$mod" = "SUPER";
          "$modAlt" = "SUPER+ALT";
          "$modCtrl" = "SUPER+CTRL";
          "$modShift" = "SUPER+SHIFT";
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
        # uwsm owns the systemd session, so disable home-manager's own unit.
        systemd.enable = false;
        xwayland.enable = true;
      };

      xdg = {
        mime.enable = true;
        portal.extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
      };
    };
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      config = mkIf config.hyprland.enable {
        asahi = {
          home = home config.hyprland.nvidia;
          module = system;
        };
        # Companion desktop modules, on by default with hyprland.
        greetd.enable = mkDefault true;
        hyprlock.enable = mkDefault true;
        hyprpolkitagent.enable = mkDefault true;
        nixos = {
          home = home config.hyprland.nvidia;
          module = system;
        };
        theme.enable = mkDefault true;
        ulauncher.enable = mkDefault true;
        waybar.enable = mkDefault true;
      };

      options.hyprland = {
        enable = mkEnableOption "hyprland desktop";
        nvidia = mkEnableOption "set nvidia GPU session variables";
      };
    }
  );
}
