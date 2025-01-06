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

  environment.systemPackages = with pkgs; [
    brave
    git
    obsidian
    vim
  ];

  hardware.pulseaudio.enable = false;

  home-manager.users.jmoore = {
    programs.yt-dlp.enable = true;
  };

  lab = {
    source = "/home/jmoore/Repos/jmoo/lab";
    users = [ "jmoore" ];
    root = true;

    ghostty.enable = true;
    greetd.enable = true;
    hyprland.enable = true;
    k3s.enable = true;
    ssh.enable = true;
    pass.enable = true;

    hypridle = {
      enable = true;
      common.monitorTimeout = null;
    };

    shell = {
      enable = true;
      root = true;
    };

    direnv = {
      enable = true;
      root = true;
    };

    vscode = {
      enable = true;
      root = true;
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

    # Enables DHCP on each ethernet and wireless interface. In case of scripted networking
    # (the default) this is the recommended approach. When using systemd-networkd it's
    # still possible to use this option, but it's recommended to use it in conjunction
    # with explicit per-interface declarations with `networking.interfaces.<interface>.useDHCP`.
    useDHCP = lib.mkDefault true;
  };

  programs = {
    steam = {
      enable = true;
      remotePlay.openFirewall = true;
      localNetworkGameTransfers.openFirewall = true;
    };

    wireshark.enable = true;
  };

  services = {
    sunshine = {
      enable = true;
      openFirewall = true;
      capSysAdmin = true;
    };

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

    printing.enable = true;

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
    ];
  };

  security.rtkit.enable = true;
}
