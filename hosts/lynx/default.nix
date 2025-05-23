{
  pkgs,
  lib,
  ...
}:
with lib;
{
  imports = [
    ../../modules/nixos.nix
    ./hardware.nix
  ];

  boot.binfmt = {
    addEmulatedSystemsToNixSandbox = true;
    emulatedSystems = [
      "aarch64-linux"
    ];
  };

  environment.systemPackages = with pkgs; [
    brave
    git
    gparted
    obsidian
    vim
  ];

  home-manager.users.jmoore = {
    programs.yt-dlp.enable = true;
    programs.ghostty.settings.theme = mkForce "Bright Lights";
  };

  lab = {
    source = "/home/jmoore/Repos/jmoo/lab";
    users = [ "jmoore" ];
    root = true;

    direnv = {
      enable = true;
      root = true;
    };

    ghostty.enable = true;
    greetd.enable = true;

    hyprpaper.enable = false;
    hyprland.enable = true;

    k3s.enable = true;

    shell = {
      enable = true;
      root = true;
    };

    ssh.enable = true;

    vscode = {
      enable = true;
      common = {
        nix.formatter = pkgs.nixfmt-rfc-style;
      };
    };
  };

  networking = {
    hostName = "lynx";

    networkmanager = {
      enable = true;
    };

    useDHCP = mkDefault true;
  };

  nix = {
    extraOptions = ''
      secret-key-files = /etc/nixos/lynx.priv
    '';

    settings = {
      trusted-users = [ "@wheel" "nix-ssh" ];
    };

    sshServe = {
      enable = true;
      write = true;
      keys = map (builtins.readFile) [
        ../meerkat/pubkeys/ssh.pub
        ./pubkeys/ssh.pub
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

    tailscale = {
      enable = true;
      useRoutingFeatures = "server";
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

}
