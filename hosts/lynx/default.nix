{
  pkgs,
  lib,
  mkHome,
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

  home-manager.users.jmoore = mkHome {
    programs.yt-dlp.enable = true;
  };

  lab = {
    source = "/home/jmoore/Repos/jmoo/lab";
    hyprland.enable = true;
    greetd.enable = true;
    k3s.enable = true;
    ssh.enable = true;
    pass.enable = true;

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

  networking.networkmanager = {
    enable = true;
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

    # xserver = {
    #   enable = true;
    #   displayManager.lightdm.enable = true;
    #   desktopManager.cinnamon.enable = true;
    #   xkb = {
    #     layout = "us";
    #     variant = "";
    #   };
    # };
  };

  programs = {
    steam = {
      enable = true;
      remotePlay.openFirewall = true;
      localNetworkGameTransfers.openFirewall = true;
    };

    wireshark.enable = true;
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
