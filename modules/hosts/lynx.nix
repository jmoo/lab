{ lib, ... }:
let
  inherit (lib) mkDefault mkForce;
in
{
  lab.hosts.lynx = {
    claude = {
      enable = true;
      skills.sensei.enable = true;
    };
    direnv.enable = true;
    ghostty.enable = true;
    hyprland = {
      enable = true;
      nvidia = true;
    };

    obsidian.sync = {
      enable = true;
      vaults = [ "/home/jmoore/Repos/jmoo/notes" ];
      service = true;
    };

    orca-slicer.enable = true;

    nixos = {
      enable = true;

      home =
        { pkgs, ... }:
        {
          home.packages = [
            pkgs.claude-loop
            pkgs.deploy-badger
            pkgs.gh
            pkgs.jq
            pkgs.opencode
            pkgs.opencode-desktop
          ];
          programs = {
            ghostty.settings.theme = mkForce "Bright Lights";
            yt-dlp.enable = true;
          };
        };

      module =
        {
          pkgs,
          config,
          ...
        }:
        {
          boot = {
            binfmt = {
              addEmulatedSystemsToNixSandbox = true;
              emulatedSystems = [
                "aarch64-linux"
              ];
            };

            initrd = {
              availableKernelModules = [
                "nvme"
                "xhci_pci"
                "ahci"
                "usbhid"
                "usb_storage"
                "sd_mod"
              ];

              luks.devices = {
                # ssd 1
                "luks-799387f6-03e2-4b22-af23-ae37d721e11f".device =
                  "/dev/disk/by-uuid/799387f6-03e2-4b22-af23-ae37d721e11f";

                # ssd 2
                "luks-3a68f4d2-1ade-4faa-be8a-f2930352214d".device =
                  "/dev/disk/by-uuid/3a68f4d2-1ade-4faa-be8a-f2930352214d";

                # swap
                "luks-324a8d98-d08c-4b34-80a4-ca9a94a88ecd".device =
                  "/dev/disk/by-uuid/324a8d98-d08c-4b34-80a4-ca9a94a88ecd";
              };
            };

            kernelModules = [ "kvm-amd" ];

            kernelParams = [
              "hid_apple.fnmode=2"
            ];

            loader = {
              efi.canTouchEfiVariables = true;
              systemd-boot.enable = true;
            };
          };

          environment.systemPackages = with pkgs; [
            anki
            brave
            btop
            dos2unix
            git
            gparted
            nudelta
            obsidian
            qFlipper
            unzip
            vim
            wev
            zip
          ];

          fileSystems = {
            "/" = {
              device = "/dev/disk/by-uuid/3ed3e47a-d611-4399-81ac-a8f47a78dabd";
              fsType = "ext4";
            };

            "/boot" = {
              device = "/dev/disk/by-uuid/8941-DA1F";
              fsType = "vfat";
              options = [
                "fmask=0077"
                "dmask=0077"
              ];
            };

            "/media/ssd1" = {
              device = "/dev/disk/by-uuid/d2380b2a-d2e4-48c9-a504-3ed0d9726f9a";
              fsType = "ext4";
            };
          };

          hardware = {
            bluetooth = {
              enable = true;
              powerOnBoot = true;
            };

            cpu.amd.updateMicrocode = mkDefault config.hardware.enableRedistributableFirmware;
            enableRedistributableFirmware = mkDefault true;
            flipperzero.enable = true;

            # Enable OpenGL
            graphics.enable = true;

            nvidia = {
              modesetting.enable = true;
              nvidiaSettings = true;
              open = false;
              package = config.boot.kernelPackages.nvidiaPackages.stable;
              powerManagement = {
                enable = false;
                finegrained = false;
              };
            };
          };

          networking = {
            hostName = "lynx";

            networkmanager.enable = true;

            useDHCP = mkDefault true;
          };

          nix.settings.trusted-users = [
            "@wheel"
          ];

          nixpkgs.hostPlatform = mkDefault "x86_64-linux";

          programs = {
            steam = {
              enable = true;
              localNetworkGameTransfers.openFirewall = true;
              remotePlay.openFirewall = true;
            };

            wireshark.enable = true;
          };

          security.rtkit.enable = true;

          services = {
            jellyfin = {
              enable = true;
              openFirewall = true;
            };

            openssh.settings.AllowUsers = [
              "nix-ssh"
              "jmoore"
            ];

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

            sunshine = {
              capSysAdmin = true;
              enable = true;
              openFirewall = true;
            };

            # nuphy air75 flashing
            udev.extraRules = ''
              KERNEL=="hidraw*", SUBSYSTEM=="hidraw", TAG+="uaccess"
            '';

            # Load nvidia driver for Xorg and Wayland
            xserver.videoDrivers = [ "nvidia" ];
          };

          swapDevices = [
            { device = "/dev/disk/by-uuid/2a722f1e-d50b-4e65-ad59-e821be7c6e05"; }
          ];

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

      system = "x86_64-linux";
    };

    shell.enable = true;
    source = "/home/jmoore/Repos/jmoo/lab";
    ssh.enable = true;

    tailscale = {
      enable = true;
      exitNode = true;
    };

    user = "jmoore";
    vscode.enable = true;
  };
}
