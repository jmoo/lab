{ ... }:
let
  # Wayland/HiDPI fixes for chromium based apps (Asahi)
  chromiumArgs = [
    "--enable-features=WaylandWindowDecorations,AllowQt"
    "--ozone-platform=wayland"
    "--gtk-version=4"
  ];
in
{
  lab.hosts.meerkat = {
    asahi = {
      enable = true;

      home =
        { pkgs, lib, ... }:
        {
          # swaylock instead of hyprlock (disabled at the host level).
          home.packages = [ pkgs.swaylock ];

          programs = {
            brave = {
              commandLineArgs = chromiumArgs;
              enable = true;
            };

            ghostty.settings.theme = lib.mkForce "Bright Lights";
            obs-studio.enable = true;
            vscode.profiles.default.userSettings = {
              "window.zoomLevel" = -3;
            };
          };

          # HiDPI settings for retina display
          wayland.windowManager.hyprland.settings = {
            "$lock" = "${lib.getExe pkgs.uwsm} app -- ${lib.getExe pkgs.swaylock}";

            bindl = [
              ",switch:on:Lid Switch,exec,systemctl suspend"
            ];

            env = [
              "GDK_SCALE,2"
              "QT_AUTO_SCREEN_SCALE_FACTOR,2"
              "QT_ENABLE_HIGHDPI_SCALING,2"
              "XCURSOR_SIZE,16"
              "HYPRCURSOR_SIZE,16"
            ];

            monitor = [
              ", highres, auto, 2"
            ];

            xwayland.force_zero_scaling = true;
          };

          # Create global config files for chromium based apps
          xdg.configFile = lib.listToAttrs (
            map
              (name: {
                inherit name;
                value.text = lib.concatStringsSep "\n" chromiumArgs;
              })
              [
                "chrome-flags.conf"
                "chromium-flags.conf"
                "electron-flags.conf"
              ]
          );
        };

      module =
        { pkgs, ... }:
        {
          environment.systemPackages = with pkgs; [
            gparted
            lm_sensors
            obsidian
            orca-slicer
          ];

          fileSystems = {
            "/" = {
              device = "/dev/disk/by-uuid/a217390b-0365-49ae-b9fa-b33118f286d5";
              fsType = "ext4";
            };

            "/boot" = {
              device = "/dev/disk/by-uuid/7720-17F9";
              fsType = "vfat";
              options = [
                "fmask=0022"
                "dmask=0022"
              ];
            };
          };

          networking = {
            hostName = "meerkat";
            networkmanager.enable = true;
          };

          services = {
            openssh.settings.AllowUsers = [ "nix-ssh" ];

            pipewire = {
              alsa = {
                enable = true;
                support32Bit = true;
              };
              enable = true;
              pulse.enable = true;
            };

            printing.enable = true;

            pulseaudio.enable = false;
          };

          users.users.jmoore = {
            description = "John Moore";
            extraGroups = [
              "networkmanager"
              "wheel"
              "dialout"
              "input"
            ];
            home = "/home/jmoore";
            isNormalUser = true;
            name = "jmoore";
          };
        };

      peripheralFirmwareHash = "sha256-mP4xKnC15rZO5+D+wexGrim/7WUg23BbjwWLDEIsrPg=";
    };

    darwin = {
      enable = true;

      home =
        { lib, ... }:
        {
          # macOS repo lives under /Users
          home.shellAliases.switch = lib.mkForce "sudo darwin-rebuild switch --flake /Users/jmoore/Repos/jmoo/lab#meerkat";
        };

      module =
        { pkgs, ... }:
        {
          environment.systemPackages = with pkgs; [ tailscale ];

          networking.hostName = "meerkat";

          users.users.jmoore = {
            home = "/Users/jmoore";
            name = "jmoore";
          };
        };
    };

    # Shared features (applied to whichever platforms are built).
    direnv.enable = true;
    ghostty.enable = true;

    # Home config applied to the user on both platforms.
    home =
      { pkgs, ... }:
      {
        home.packages = with pkgs; [
          bat
          binwalk
          claude-code
          colordiff
          gh
          git
          radare2
          tio
          vbindiff
        ];

        programs.yt-dlp.enable = true;
      };

    # Linux-only features (forLinux → only affect the asahi build).
    hyprland.enable = true;

    # hyprlock currently segfaults on asahi; use swaylock instead (see asahi.home).
    hyprlock.enable = false;

    # Darwin-only feature.
    iterm2.enable = true;

    shell.enable = true;
    source = "/home/jmoore/Repos/jmoo/lab";
    ssh.enable = true;
    tailscale.enable = true;
    user = "jmoore";
    vscode.enable = true;
  };
}
