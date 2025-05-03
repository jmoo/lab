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

    iwmenu.enable = true;

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
      enable = false;
    };

    useDHCP = mkDefault true;
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
