{ ... }:
{
  lab.hosts.lynx = {
    user = "jmoore";
    source = "/home/jmoore/Repos/jmoo/lab";

    direnv.enable = true;
    ghostty.enable = true;
    greetd.enable = true;
    hyprland.enable = true;
    shell.enable = true;
    ssh.enable = true;
    vscode.enable = true;

    tailscale = {
      enable = true;
      exitNode = true;
    };

    nixos = {
      enable = true;
      system = "x86_64-linux";

      home =
        { lib, ... }:
        {
          # nvidia GPU
          hyprland.nvidia = true;

          programs = {
            claude-code.enable = true;
            ghostty.settings.theme = lib.mkForce "Bright Lights";
            yt-dlp.enable = true;
          };
        };

      module =
        {
          pkgs,
          lib,
          config,
          modulesPath,
          ...
        }:
        {
          imports = [
            (modulesPath + "/installer/scan/not-detected.nix")
          ];

          # ---- hardware (was hosts/lynx/hardware.nix) ----

          # Bootloader.
          boot.loader.systemd-boot.enable = true;
          boot.loader.efi.canTouchEfiVariables = true;

          boot.initrd.availableKernelModules = [
            "nvme"
            "xhci_pci"
            "ahci"
            "usbhid"
            "usb_storage"
            "sd_mod"
          ];

          boot.kernelParams = [
            "hid_apple.fnmode=2"
          ];

          boot.initrd.kernelModules = [ ];
          boot.kernelModules = [ "kvm-amd" ];
          boot.extraModulePackages = [ ];

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

          boot.initrd.luks.devices = {
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

          swapDevices = [
            { device = "/dev/disk/by-uuid/2a722f1e-d50b-4e65-ad59-e821be7c6e05"; }
          ];

          nixpkgs.hostPlatform = lib.mkDefault "x86_64-linux";
          hardware.cpu.amd.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;

          # Enable OpenGL
          hardware.graphics = {
            enable = true;
          };

          services.udev.extraRules = ''
            KERNEL=="hidraw*", SUBSYSTEM=="hidraw", TAG+="uaccess"
          '';

          # Load nvidia driver for Xorg and Wayland
          services.xserver.videoDrivers = [ "nvidia" ];

          hardware.bluetooth.enable = true;
          hardware.bluetooth.powerOnBoot = true;

          hardware.nvidia = {
            modesetting.enable = true;
            powerManagement.enable = false;
            powerManagement.finegrained = false;
            open = false;
            nvidiaSettings = true;
            package = config.boot.kernelPackages.nvidiaPackages.stable;
          };

          # ---- host configuration (was hosts/lynx/default.nix) ----

          environment.systemPackages = with pkgs; [
            anki
            brave
            dos2unix
            git
            gparted
            nudelta
            obsidian
            vim
            wev
            zip
          ];

          networking = {
            hostName = "lynx";

            networkmanager = {
              enable = true;
            };

            useDHCP = lib.mkDefault true;
          };

          nix = {
            extraOptions = ''
              secret-key-files = /etc/nixos/lynx.priv
            '';

            settings = {
              trusted-users = [
                "@wheel"
                "nix-ssh"
              ];
            };

            sshServe = {
              enable = true;
              write = true;
              keys = map (builtins.readFile) [
                ../keys/meerkat-ssh.pub
                ../keys/lynx-ssh.pub
              ];
            };
          };

          programs = {
            steam = {
              enable = true;
              remotePlay.openFirewall = true;
              localNetworkGameTransfers.openFirewall = true;
            };

            wireshark.enable = true;
          };

          security.rtkit.enable = true;

          services = {
            jellyfin = {
              enable = true;
              openFirewall = true;
            };

            pipewire = {
              enable = true;
              alsa.enable = true;
              alsa.support32Bit = true;
              pulse.enable = true;
            };

            openssh.settings.AllowUsers = [ "nix-ssh" ];

            printing.enable = true;

            pulseaudio.enable = false;

            sunshine = {
              enable = true;
              openFirewall = true;
              capSysAdmin = true;
            };
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
        };
    };
  };
}
