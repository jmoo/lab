{
  config,
  lib,
  pkgs,
  ...
}:
with lib;
let
  wrapHyprCommand = x: "${optionalString config.lab.hyprland.uwsm "${getExe pkgs.uwsm} app -- "}${x}";
in
{
  options.lab = {
    hyprland = {
      enable = mkEnableOption "Enable hyprland home-manager configuration";
      nvidia = mkEnableOption "Enable nvidia";
      uwsm = mkEnableOption "Enable uwsm";
    };

    hyprlock = {
      enable = mkEnableOption "Enable hyprlock home-manager configuration";
    };

    hypridle = {
      enable = mkEnableOption "Enable hypridle home-manager configuration";

      lockCommand = mkOption {
        type = types.str;
        default = wrapHyprCommand config.lab.apps.lock.command;
      };

      lockTimeout = mkOption {
        type = with types; nullOr number;
        default = 900;
      };

      monitorTimeout = mkOption {
        type = with types; nullOr number;
        default = 1200;
      };
    };
  };

  config = mkMerge [
    # Hyprland
    (mkIf config.lab.hyprland.enable {
      home.packages =
        with pkgs;
        [
          hyprpolkitagent
          playerctl
          brightnessctl
        ]

        # Add default apps to the environment
        ++ (map (x: x.package) (attrValues config.lab.apps));

      lab = {
        hyprlock.enable = true;
        hypridle.enable = true;
        theme.enable = true;
        ulauncher.enable = true;
        waybar.enable = true;
      };

      # Always have kitty as a backup
      programs.kitty.enable = true;

      wayland.windowManager.hyprland = {
        enable = true;
        extraConfig = builtins.readFile ./hyprland.conf;
        settings = mkMerge [
          {
            "$mod" = mkDefault "SUPER";
            "$modCtrl" = mkDefault "SUPER+CTRL";
            "$modAlt" = mkDefault "SUPER+ALT";
            "$modShift" = mkDefault "SUPER+SHIFT";
            "$modShiftCtrl" = mkDefault "SUPER+SHIFT+CTRL";

            env = mkIf config.lab.hyprland.nvidia [
              "LIBVA_DRIVER_NAME,nvidia"
              "__GLX_VENDOR_LIBRARY_NAME,nvidia"
            ];
          }

          # Add variables for default apps
          (mapAttrs' (n: v: {
            name = "\$${n}";
            value = mkDefault (wrapHyprCommand v.command);
          }) config.lab.apps)
        ];

        xwayland.enable = true;
      };
    })

    # UWSM
    (mkIf config.lab.hyprland.uwsm {
      lab = {
        ulauncher.package = mkIf config.lab.hyprland.uwsm pkgs.ulauncher-uwsm;
        waybar.settings = {
          bluetooth.on-click = wrapHyprCommand config.lab.apps.bluetoothManager.command;
          pulseaudio.on-click = wrapHyprCommand config.lab.apps.audioManager.command;
        };
      };

      wayland.windowManager.hyprland.systemd.enable = mkForce false;
    })

    # Hyprlock
    (mkIf config.lab.hyprlock.enable {
      lab.apps.lock.package = config.programs.hyprlock.package;

      programs.hyprlock = {
        enable = true;
        extraConfig = builtins.readFile ./hyprlock.conf;
      };
    })

    # Hypridle
    (mkIf config.lab.hypridle.enable {
      services.hypridle = {
        enable = true;

        settings = {
          general = {
            after_sleep_cmd = "hyprctl dispatch dpms on";
            ignore_dbus_inhibit = false;
            lock_cmd = config.lab.hypridle.lockCommand;
          };

          listener =
            (lists.optional (config.lab.hypridle.lockTimeout != null) {
              timeout = config.lab.hypridle.lockTimeout;
              on-timeout = config.lab.hypridle.lockCommand;
            })
            ++ (lists.optional (config.lab.hypridle.monitorTimeout != null) {
              timeout = config.lab.hypridle.monitorTimeout;
              on-timeout = "hyprctl dispatch dpms off";
              on-resume = "hyprctl dispatch dpms on";
            });
        };
      };
    })
  ];
}
