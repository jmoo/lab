{ pkgs, lib, ... }:
with lib;
{
  imports = [ ../../modules/nixos.nix ];

  environment.systemPackages = with pkgs; [
    git
    gparted
    vim
  ];

  lab = {
    name = "axolotl";
    source = "/home/jmoore/Repos/jmoore/home";
    users = [ "jmoore" ];
    root = true;

    direnv = {
      enable = true;
      root = true;
    };

    ghostty.enable = true;
    greetd.enable = true;
    hyprpaper.enable = true;

    hyprland = {
      enable = true;
      common = {
        wallpapers = [
          { source = ../../resources/wallpaper/5120x1440_a.png; }
          { source = ../../resources/wallpaper/5120x1440_b.png; }
          { source = ../../resources/wallpaper/5120x1440_c.jpg; }
        ];
      };
    };

    shell = {
      enable = true;
      root = true;
    };

    ssh.enable = true;

    vscode = {
      enable = true;
      common = {
        nix.formatter = pkgs.nixfmt;
      };
    };
  };

  networking = {
    hostName = "axolotl";

    networkmanager = {
      enable = true;
    };

    useDHCP = mkDefault true;
  };

  programs = {
    wireshark.enable = true;
  };

  security.rtkit.enable = true;

  services = {
    pipewire = {
      enable = true;
      alsa.enable = true;
      alsa.support32Bit = true;
      pulse.enable = true;
    };

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
}
