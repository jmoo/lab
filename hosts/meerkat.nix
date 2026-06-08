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
    user = "jmoore";
    source = "/home/jmoore/Repos/jmoo/lab";

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

    # Shared features (applied to whichever platforms are built).
    direnv.enable = true;
    ghostty.enable = true;
    shell.enable = true;
    vscode.enable = true;

    # Linux-only features (forLinux → only affect the asahi build).
    greetd.enable = true;
    hyprland.enable = true;
    ssh.enable = true;
    tailscale.enable = true;

    # Darwin-only feature.
    iterm2.enable = true;

    asahi = {
      enable = true;
      peripheralFirmwareHash = "sha256-mP4xKnC15rZO5+D+wexGrim/7WUg23BbjwWLDEIsrPg=";

      home =
        { pkgs, lib, ... }:
        {
          # Hyprlock currently segfaults on asahi; use swaylock instead.
          hyprlock.enable = false;
          apps.lock.package = lib.mkForce pkgs.swaylock;

          programs = {
            brave = {
              enable = true;
              commandLineArgs = chromiumArgs;
            };

            ghostty.settings.theme = lib.mkForce "Bright Lights";
            obs-studio.enable = true;
            vscode.profiles.default.userSettings = {
              "window.zoomLevel" = -3;
            };
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

          # HiDPI settings for retina display
          wayland.windowManager.hyprland = {
            settings = {
              monitor = [
                ", highres, auto, 2"
              ];

              xwayland = {
                force_zero_scaling = true;
              };

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
            };
          };
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

          networking = {
            hostName = "meerkat";
            networkmanager.enable = true;
          };

          services = {
            pipewire = {
              enable = true;
              alsa.enable = true;
              alsa.support32Bit = true;
              pulse.enable = true;
            };

            openssh.settings.AllowUsers = [ "nix-ssh" ];

            printing.enable = true;

            pulseaudio.enable = false;
          };

          users.users.jmoore = {
            name = "jmoore";
            home = "/home/jmoore";
            isNormalUser = true;
            description = "John Moore";
            extraGroups = [
              "networkmanager"
              "wheel"
              "dialout"
              "input"
            ];
          };

          fileSystems."/" = {
            device = "/dev/disk/by-uuid/a217390b-0365-49ae-b9fa-b33118f286d5";
            fsType = "ext4";
          };

          fileSystems."/boot" = {
            device = "/dev/disk/by-uuid/7720-17F9";
            fsType = "vfat";
            options = [
              "fmask=0022"
              "dmask=0022"
            ];
          };
        };
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
          networking.hostName = "meerkat";

          environment.systemPackages = with pkgs; [ tailscale ];

          users.users.jmoore = {
            name = "jmoore";
            home = "/Users/jmoore";
          };
        };
    };
  };
}
