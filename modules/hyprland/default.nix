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

  # Home config for the hyprland session. `nvidia` adds GPU session variables;
  # `lua` picks hyprland.lua over hyprland.conf.
  home =
    { nvidia, lua }:
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

      env = [
        [
          "QT_QPA_PLATFORM"
          "wayland"
        ]
      ]
      ++ lib.optionals nvidia [
        [
          "LIBVA_DRIVER_NAME"
          "nvidia"
        ]
        [
          "__GLX_VENDOR_LIBRARY_NAME"
          "nvidia"
        ]
      ];

      # Variables the config files reference: `$name` in hyprlang, a local in lua.
      vars = {
        mod = "SUPER";
        modAlt = "SUPER+ALT";
        modCtrl = "SUPER+CTRL";
        modShift = "SUPER+SHIFT";
        modShiftCtrl = "SUPER+SHIFT+CTRL";
        terminal = uwsm terminal;
      };

      hyprlang = {
        extraConfig = builtins.readFile ./hyprland.conf;
        settings = lib.mapAttrs' (name: lib.nameValuePair "\$${name}") vars // {
          env = map (lib.concatStringsSep ",") env;
        };
      };

      luaConfig = {
        # ⚠️ Explicit: the default follows `home.stateVersion`, and below 26.05 that
        # is hyprlang, which writes a hyprland.conf Hyprland ≥ 0.57 never reads.
        configType = "lua";
        extraConfig = builtins.readFile ./hyprland.lua;
        settings = lib.mapAttrs (_: value: { _var = value; }) vars // {
          env = map (pair: { _args = pair; }) env;
        };
      };
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
        # uwsm owns the systemd session, so disable home-manager's own unit.
        systemd.enable = false;
        xwayland.enable = true;
      }
      // (if lua then luaConfig else hyprlang);

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
        # Asahi's pinned Hyprland (and home-manager) predate lua config.
        asahi = {
          home = home {
            inherit (config.hyprland) nvidia;
            lua = false;
          };
          module = system;
        };
        # Companion desktop modules, on by default with hyprland.
        greetd.enable = mkDefault true;
        hyprlock.enable = mkDefault true;
        hyprpolkitagent.enable = mkDefault true;
        nixos = {
          home = home {
            inherit (config.hyprland) nvidia;
            lua = true;
          };
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
