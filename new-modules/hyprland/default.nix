{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule;
  inherit (lib') mkEnableOption mkIf;

  # ---------------------------------------------------------------------------
  # Home-manager modules that together make up the desktop. They are imported
  # into a single home-manager evaluation, so they share the `apps` option set
  # and the `wrapHyprCommand` module arg. Internal options live off of `lab`.
  # ---------------------------------------------------------------------------

  apps =
    {
      pkgs,
      config,
      lib,
      ...
    }:
    with lib;
    {
      options.apps = mkOption {
        description = "Default apps";
        type =
          with types;
          attrsOf (
            submodule (
              { name, config, ... }:
              {
                options = {
                  enable = (mkEnableOption "Enable default app") // {
                    default = true;
                  };

                  name = mkOption {
                    type = types.str;
                    default = name;
                  };

                  package = mkOption {
                    type = with types; nullOr package;
                    default = null;
                  };

                  command = mkOption {
                    type = types.str;
                    default = getExe config.package;
                  };
                };
              }
            )
          );
        default = { };
      };

      config.apps = with pkgs; {
        terminal.package = mkDefault (
          if config.programs.ghostty.enable then config.programs.ghostty.package else xterm
        );
        bluetoothManager = {
          package = mkDefault blueman;
          command = "${blueman}/bin/blueman-manager";
        };
        audioManager.package = mkDefault pavucontrol;
        launcher.package = mkDefault config.ulauncher.package;
        displayManager.package = mkDefault wdisplays;
        fileManager.package = mkDefault nemo;
      };
    };

  hyprland =
    {
      config,
      lib,
      pkgs,
      wrapHyprCommand,
      ...
    }:
    with lib;
    {
      options.hyprland = {
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

      config = mkMerge [
        {
          # Make QT apps happy
          hyprland.sessionVariables = {
            QT_QPA_PLATFORM = "wayland";
          };

          # Wrapping all executables called from hyprland. This will wrap
          # everything with uwsm calls if uwsm is enabled.
          _module.args.wrapHyprCommand =
            x:
            "${
              optionalString (config.hyprland.enable && config.hyprland.uwsm) "${getExe pkgs.uwsm} app -- "
            }${x}";
        }

        # Hyprland
        (mkIf config.hyprland.enable {
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
              ++ (map (x: x.package) (filter (x: x.enable && isDerivation x.package) (attrValues config.apps)));

            sessionVariables = config.hyprland.sessionVariables;
          };

          # Enable default programs and services for a complete
          # out of the box hyprland experience.
          hyprlock.enable = mkDefault true;
          hypridle.enable = mkDefault false;
          hyprpaper.enable = mkDefault false;
          hyprpolkitagent.enable = mkDefault true;
          theme.enable = mkDefault true;
          ulauncher.enable = mkDefault true;
          waybar.enable = mkDefault true;

          services = {
            # Notification daemon
            swaync.enable = mkDefault true;

            # Screenshot application
            flameshot.enable = mkDefault true;

            # NetworkManager tray applet
            network-manager-applet.enable = mkDefault true;
          };

          systemd.user = {
            sessionVariables = config.hyprland.sessionVariables;

            services.network-manager-applet.Unit = {
              After = [ "graphical-session.target" ];
              PartOf = [ "graphical-session.target" ];
            };
          };

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
                env = mapAttrsToList (n: v: "${n},${v}") config.hyprland.sessionVariables;

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
              }) config.apps)
            ];

            xwayland.enable = true;
          };

          xdg = {
            mime.enable = true;
            portal.extraPortals = with pkgs; [
              xdg-desktop-portal-gtk
            ];
          };
        })

        # Environment flags for nvidia GPUs
        (mkIf config.hyprland.nvidia {
          hyprland.sessionVariables = {
            LIBVA_DRIVER_NAME = "nvidia";
            __GLX_VENDOR_LIBRARY_NAME = "nvidia";
          };
        })

        # UWSM - Manages graphical-session systemd user targets and scopes
        (mkIf config.hyprland.uwsm {
          # Patched version of ulauncher that launches everything with uwsm
          ulauncher.package = mkIf config.hyprland.uwsm pkgs.ulauncher-uwsm;

          # Override waybar launcher commands with uwsm variants
          waybar.settings = {
            bluetooth.on-click = wrapHyprCommand config.apps.bluetoothManager.command;
            pulseaudio.on-click = wrapHyprCommand config.apps.audioManager.command;
          };

          # Hyprland is started by UWSM so we need to disable the systemd service
          wayland.windowManager.hyprland.systemd.enable = mkForce false;
        })
      ];
    };

  hypridle =
    {
      config,
      lib,
      pkgs,
      wrapHyprCommand,
      ...
    }:
    with lib;
    {
      options.hypridle = {
        enable = mkEnableOption "Enable hypridle for inactivity timeout management in home-manager";

        lockCommand = mkOption {
          description = "Command that triggers the lock screen";
          type = types.str;
          default = wrapHyprCommand config.apps.lock.command;
        };

        suspendCommand = mkOption {
          description = "Command that triggers the systemd suspend/sleep";
          type = types.str;
          default = wrapHyprCommand config.apps.suspend.command;
        };

        displayCommand = {
          on = mkOption {
            description = "Command that enables display";
            type = types.str;
            default = wrapHyprCommand config.apps.displayOn.command;
          };

          off = mkOption {
            description = "Command that enables display";
            type = types.str;
            default = wrapHyprCommand config.apps.displayOff.command;
          };
        };

        lockTimeout = mkOption {
          description = "Seconds until lock screen activates";
          type = with types; nullOr number;
          default = 900;
        };

        monitorTimeout = mkOption {
          description = "Seconds until monitors are turned off";
          type = with types; nullOr number;
          default = 1200;
        };

        suspendTimeout = mkOption {
          description = "Seconds until device goes to sleep";
          type = with types; nullOr number;
          default = 1201;
        };
      };

      config = mkIf config.hypridle.enable {
        home.packages = with pkgs; [
          nixos-artwork.wallpapers.dracula
        ];

        apps = {
          displayOn = mkDefault {
            command = "hyprctl dispatch dpms on";
          };

          displayOff = mkDefault {
            command = "hyprctl dispatch dpms off";
          };

          suspend = mkDefault {
            command = "systemctl suspend";
          };
        };

        services = {
          hypridle = {
            enable = true;

            settings = {
              general = {
                after_sleep_cmd = config.hypridle.displayCommand.on;
                before_sleep_cmd = config.hypridle.lockCommand;
                ignore_dbus_inhibit = false;
                lock_cmd = config.hypridle.lockCommand;
                inhibit_sleep = 3;
              };

              listener =
                (lists.optional (config.hypridle.lockTimeout != null) {
                  timeout = config.hypridle.lockTimeout;
                  on-timeout = config.hypridle.lockCommand;
                })
                ++ (lists.optional (config.hypridle.monitorTimeout != null) {
                  timeout = config.hypridle.monitorTimeout;
                  on-timeout = config.hypridle.displayCommand.on;
                  on-resume = config.hypridle.displayCommand.off;
                })
                ++ (lists.optional (config.hypridle.suspendTimeout != null) {
                  timeout = config.hypridle.suspendTimeout;
                  on-timeout = config.hypridle.suspendCommand;
                });
            };
          };
        };
      };
    };

  hyprlock =
    { config, lib, ... }:
    with lib;
    {
      options.hyprlock = {
        enable = mkEnableOption "Use hyprlock as the default lock screen in home-manager";
      };

      config = mkIf config.hyprlock.enable {
        apps.lock.package = config.programs.hyprlock.package;

        programs.hyprlock = {
          enable = true;
          extraConfig = builtins.readFile ./config/hyprlock.conf;
        };
      };
    };

  hyprpaper =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    with lib;
    let
      wallpapers = filter (
        x:
        x.enable
        && elem x.type [
          "desktop"
          "lock_and_desktop"
        ]
      ) config.hyprland.wallpapers;

      static = filter (x: x.monitors != null) wallpapers;
      dynamic = filter (x: x.monitors == null) wallpapers;
      preload = filter (x: x.preload) wallpapers;
      monitors = concatLists (map (x: if x.monitors == null then [ ] else x.monitors) wallpapers);
      source = x: "${optionalString (x.mode != "cover") "${x.mode}:"}${toString x.source}";
    in
    {
      options.hyprpaper = {
        enable = mkEnableOption "Enable hyprpaper for wallpaper management in home-manager";
      };

      config = mkIf config.hyprpaper.enable (mkMerge [
        {
          services.hyprpaper = {
            enable = true;

            settings = {
              ipc = "on";
              splash = false;
              preload = map (x: toString x.source) preload;
              wallpapers =
                (map (x: " , ${source x}") dynamic)
                ++ (concatLists (map (x: map (m: "${m}, ${source x}") x.monitors) static));
            };
          };

          systemd.user.services.hyprpaper.Service.ExecStartPost = pkgs.writeScript "hyprpaper-apply" ''
            #!/usr/bin/env bash
            set -euxo pipefail
            sleep 2
            monitors=(${concatStringsSep " " (map (x: ''"${source x}"'') monitors)})
            wallpapers=(${concatStringsSep " " (map (x: ''"${source x}"'') dynamic)})
            selected=''${wallpapers[ $RANDOM % ''${#wallpapers[@]} ]}

            echo "Monitors: ''${monitors[*]}"
            echo "Wallpapers: ''${wallpapers[*]}"
            echo "Selected: ''${selected}"

            for monitor in $(hyprctl monitors | grep 'Monitor' | awk '{ print $2 }'); do
              if [[ ! " ''${monitors[*]} " =~ [[:space:]]''${monitor}[[:space:]] ]]; then
                  echo "Setting wallpaper \"$selected\" for monitor \"$monitor\""
                  hyprctl hyprpaper wallpaper "$monitor,$selected"
              else
                echo "Wallpaper already set for monitor \"$monitor\""
              fi
            done
          '';

          wayland.windowManager.hyprland.settings = {
            # Disable default anime wallpapers
            misc = {
              force_default_wallpaper = 0;
              disable_hyprland_logo = true;
            };
          };
        }
      ]);
    };

  hyprpolkitagent =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    with lib;
    {
      options.hyprpolkitagent = {
        enable = mkEnableOption "Use hyprpolkitagent as the default polkit password prompt service in home-manager";

        package = mkOption {
          type = types.package;
          default = pkgs.hyprpolkitagent;
        };
      };

      config = mkIf config.hyprpolkitagent.enable {
        home.packages = [
          config.hyprpolkitagent.package
        ];

        # Hyprpolkitagent already ships with a systemd service, we just need
        # to put it in the correct place so it will auto-start
        xdg.configFile."systemd/user/graphical-session.target.wants/hyprpolkitagent.service".source =
          "${config.hyprpolkitagent.package}/share/systemd/user/hyprpolkitagent.service";
      };
    };

  theme =
    {
      pkgs,
      lib,
      config,
      ...
    }:
    with lib;
    {
      options.theme.enable = mkEnableOption "Enable theme";

      config = mkIf config.theme.enable {
        home.packages = with pkgs; [
          # Theme
          adwaita-icon-theme
          adwaita-qt
          adwaita-qt6
          adw-gtk3

          # Fonts
          nerd-fonts.ubuntu
          nerd-fonts.ubuntu-mono
          font-awesome
        ];

        dconf = {
          enable = true;
          settings = {
            "org/gnome/desktop/interface" = {
              color-scheme = "prefer-dark";
            };
          };
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

          gtk4.theme = config.gtk.theme;
        };

        qt = {
          enable = true;
          style.name = "adwaita-dark";
          platformTheme.name = "adwaita";
        };
      };
    };

  ulauncher =
    {
      pkgs,
      config,
      lib,
      ...
    }:
    with lib;
    {
      options.ulauncher = {
        enable = mkEnableOption "Enable ulauncher home-manager configuration";
        package = mkOption {
          type = types.package;
          default = pkgs.ulauncher;
        };
      };

      config = mkIf config.ulauncher.enable {
        home.packages = [ config.ulauncher.package ];

        apps.launcher.package = config.ulauncher.package;

        systemd.user.services.ulauncher = {
          Unit.Description = "Ulauncher - Application Runner";
          Install.WantedBy = [ "graphical-session.target" ];
          Unit = {
            After = [ "graphical-session.target" ];
            PartOf = [ "graphical-session.target" ];
            ConditionEnvironment = "WAYLAND_DISPLAY";
          };
          Service = {
            ExecStart = "${getExe config.ulauncher.package}";
            Restart = "on-failure";
          };
        };
      };
    };

  waybar =
    { lib, config, ... }:
    with lib;
    {
      options.waybar = {
        enable = mkEnableOption "Enable waybar home-manager configuration";

        settings = mkOption {
          type = with types; attrsOf anything;
          default = { };
        };
      };

      config = mkIf config.waybar.enable {
        apps.bar.package = config.programs.waybar.package;

        waybar.settings = mkMerge [
          (with builtins; fromJSON (readFile ./waybar/config.json))
          {
            bluetooth.on-click = mkDefault config.apps.bluetoothManager.command;
            pulseaudio.on-click = mkDefault config.apps.audioManager.command;
          }
        ];

        programs.waybar = {
          enable = true;
          style = builtins.readFile ./waybar/style.css;
          settings = [
            config.waybar.settings
          ];
        };

        systemd.user.services.waybar = {
          Unit.Description = "waybar";
          Install.WantedBy = [ "graphical-session.target" ];
          Unit = {
            After = [ "graphical-session.target" ];
            PartOf = [ "graphical-session.target" ];
            ConditionEnvironment = "WAYLAND_DISPLAY";
          };
          Service = {
            ExecStart = "${getExe config.programs.waybar.package}";
            Restart = "on-failure";
          };
        };
      };
    };

  # The full desktop home-manager bundle.
  desktop = {
    imports = [
      apps
      hyprland
      hypridle
      hyprlock
      hyprpaper
      hyprpolkitagent
      theme
      ulauncher
      waybar
    ];

    hyprland = {
      enable = true;
      uwsm = true;
    };
  };

  # System (NixOS) configuration enabling hyprland session-wide.
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
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.hyprland.enable = mkEnableOption "hyprland desktop";

      config = mkIf config.hyprland.enable {
        nixos = {
          module = system;
          home = desktop;
        };
        asahi = {
          module = system;
          home = desktop;
        };
      };
    }
  );
}
