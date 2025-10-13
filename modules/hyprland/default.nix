{
  config,
  lib,
  pkgs,
  wrapHyprCommand,
  ...
}:
with lib;
{
  imports = [
    ./hypridle.nix
    ./hyprlock.nix
    ./hyprpaper.nix
    ./hyprpolkitagent.nix
  ];

  options.lab = {
    hyprland = {
      enable = mkEnableOption "Enable hyprland home-manager configuration";

      nvidia = mkEnableOption "Set to true if using an nvidia gpu";

      sessionVariables = mkOption {
        description = "Session variables that need to be included in the hyprland session.";
        type = with types; attrsOf str;
        default = { };
      };

      uwsm = mkEnableOption "Manages graphical-session systemd user targets and scopes with uwsm";

      wallpapers = mkOption {
        type = types.listOf (
          types.submodule ({
            options = {
              enable = (mkEnableOption "Enable the wallpaper") // {
                default = true;
              };

              source = mkOption {
                description = "Wallpaper source";
                type =
                  with types;
                  oneOf [
                    package
                    path
                  ];
              };

              preload = mkOption {
                description = "Preload the wallpaper";
                type = types.bool;
                default = true;
              };

              mode = mkOption {
                description = "Wallpaper display type";
                type = types.enum [
                  "cover"
                  "contain"
                  "tile"
                ];
                default = "cover";
              };

              monitors = mkOption {
                description = "Only apply wallpaper to these monitors";
                type = with types; nullOr (listOf str);
                default = null;
              };

              type = mkOption {
                description = "Type of wallpaper";
                type =
                  with types;
                  enum [
                    "lock"
                    "desktop"
                    "lock_and_desktop"
                  ];
                default = "desktop";
              };
            };
          })
        );
        default = [ ];
      };
    };
  };

  config = mkMerge [
    {
      # Make QT apps happy
      lab.hyprland.sessionVariables = {
        QT_QPA_PLATFORM = "wayland";
      };

      # Wrapping all executables called from hyprland. This will wrap
      # everything with uwsm calls if uwsm is enabled.
      _module.args.wrapHyprCommand =
        x:
        "${
          optionalString (
            config.lab.hyprland.enable && config.lab.hyprland.uwsm
          ) "${getExe pkgs.uwsm} app -- "
        }${x}";
    }

    # Hyprland
    (mkIf config.lab.hyprland.enable {
      home = {
        packages =
          with pkgs;
          [
            # Media and brightness controls
            brightnessctl
            playerctl

            # Include xterm so there is always a non-gpu accelerated terminal available
            xterm
          ]

          # Add default apps to the environment
          ++ (map (x: x.package) (
            filter (x: x.enable && isDerivation x.package) (attrValues config.lab.apps)
          ));

        sessionVariables = config.lab.hyprland.sessionVariables;
      };

      # Enable default programs and services for a complete
      # out of the box hyprland experience.
      lab = {
        hyprlock.enable = mkDefault true;
        hypridle.enable = mkDefault true;
        hyprpaper.enable = mkDefault false;
        hyprpolkitagent.enable = mkDefault true;
        theme.enable = mkDefault true;
        ulauncher.enable = mkDefault true;
        waybar.enable = mkDefault true;
      };

      # Notification daemon
      services.swaync.enable = mkDefault true;

      systemd.user.sessionVariables = config.lab.hyprland.sessionVariables;

      wayland.windowManager.hyprland = {
        enable = true;
        extraConfig = builtins.readFile ./config/hyprland.conf;
        settings = mkMerge [
          {
            # Set default mod variables
            "$mod" = mkDefault "SUPER";
            "$modCtrl" = mkDefault "SUPER+CTRL";
            "$modAlt" = mkDefault "SUPER+ALT";
            "$modShift" = mkDefault "SUPER+SHIFT";
            "$modShiftCtrl" = mkDefault "SUPER+SHIFT+CTRL";

            # Set sessionVariables
            env = mapAttrsToList (n: v: "${n},${v}") config.lab.hyprland.sessionVariables;

            # Enable default anime wallpapers
            misc = {
              force_default_wallpaper = mkDefault (-1);
              disable_hyprland_logo = mkDefault false;
            };
          }

          # Set variables for default apps
          (mapAttrs' (n: v: {
            name = "\$${n}";
            value = mkDefault (wrapHyprCommand v.command);
          }) config.lab.apps)
        ];

        xwayland.enable = true;
      };
    })

    # Environment flags for nvidia GPUs
    (mkIf config.lab.hyprland.nvidia {
      lab.hyprland.sessionVariables = {
        LIBVA_DRIVER_NAME = "nvidia";
        __GLX_VENDOR_LIBRARY_NAME = "nvidia";
      };
    })

    # UWSM - Manages graphical-session systemd user targets and scopes
    (mkIf config.lab.hyprland.uwsm {
      lab = {
        # Patched version of ulauncher that launches everything with uwsm
        ulauncher.package = mkIf config.lab.hyprland.uwsm pkgs.ulauncher-uwsm;

        # Override waybar launcher commands with uwsm variants
        waybar.settings = {
          bluetooth.on-click = wrapHyprCommand config.lab.apps.bluetoothManager.command;
          pulseaudio.on-click = wrapHyprCommand config.lab.apps.audioManager.command;
        };
      };

      # Hyprland is started by UWSM so we need to disable the systemd service
      wayland.windowManager.hyprland.systemd.enable = mkForce false;
    })
  ];
}
