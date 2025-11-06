{ pkgs, lib, ... }:
let
  inherit (lib) mkForce listToAttrs concatStringsSep;

  # Wayland/HiDPI fixes for chromium based apps
  chromiumArgs = [
    "--enable-features=WaylandWindowDecorations,AllowQt"
    "--ozone-platform=wayland"
    "--gtk-version=4"
  ];
in
{
  imports = [
    ../../modules/asahi.nix
    ./common.nix
  ];

  environment.systemPackages = with pkgs; [
    gparted
    lm_sensors
    obsidian
    orca-slicer
  ];

  home-manager.users.jmoore = {
    programs = {
      brave = {
        enable = true;
        commandLineArgs = chromiumArgs;
      };

      ghostty.settings.theme = mkForce "Bright Lights";
      obs-studio.enable = true;
      vscode.profiles.default.userSettings = {
        "window.zoomLevel" = -3;
      };
    };

    # Create global config files for chromium based apps
    xdg.configFile = listToAttrs (
      map
        (name: {
          inherit name;
          value.text = concatStringsSep "\n" chromiumArgs;
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

  lab = {
    asahi.peripheralFirmwareHash = "sha256-mP4xKnC15rZO5+D+wexGrim/7WUg23BbjwWLDEIsrPg=";
    ghostty.enable = true;
    greetd.enable = true;
    hyprpaper.enable = false;
    hyprland.enable = true;
    ssh.enable = true;

    # Hyprlock currently segfaults in asahi
    # Use swaylock instead until PR merged.
    hyprlock.enable = false;
    apps.common.lock = mkForce {
      package = pkgs.swaylock;
    };
  };

  networking = {
    networkmanager = {
      enable = true;
    };
  };

  services = {
    tailscale.enable = true;

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
}
