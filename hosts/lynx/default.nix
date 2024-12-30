{
  config,
  pkgs,
  mkHome,
  ...
}:
{
  imports = [
    ../../modules/nixos.nix
    ./hardware.nix
  ];

  environment.systemPackages = with pkgs; [
    brave
    git
    vim
  ];

  hardware.pulseaudio.enable = false;

  home-manager.users.jmoore = mkHome {
    home = with config.users.users.jmoore; {
      homeDirectory = home;
      username = name;
      shellAliases = {
        switch = "sudo nixos-rebuild switch --flake /home/jmoore/Repos/jmoo/lab#lynx";
      };
    };

    programs = {
      yt-dlp.enable = true;
    };
  };

  lab = {
    direnv.enable = true;
    hyprland.enable = true;
    k3s.enable = true;
    shell.enable = true;
    vscode = {
      enable = true;
      common.nix.formatter = pkgs.nixfmt-rfc-style;
    };
  };

  networking = {
    firewall.allowedTCPPorts = [
      22
    ];
    networkmanager = {
      enable = true;
    };
  };

  services = {
    jellyfin = {
      enable = true;
      openFirewall = true;
    };

    openssh = {
      enable = true;
      ports = [ 22 ];
      settings = {
        PasswordAuthentication = true;
        AllowUsers = [ "jmoore" ];
        UseDns = true;
        X11Forwarding = false;
        PermitRootLogin = "no";
      };
    };

    pipewire = {
      enable = true;
      alsa.enable = true;
      alsa.support32Bit = true;
      pulse.enable = true;
    };

    printing.enable = true;

    xserver = {
      enable = true;
      displayManager.lightdm.enable = true;
      desktopManager.cinnamon.enable = true;
      xkb = {
        layout = "us";
        variant = "";
      };
    };
  };

  # systemd.services = {
  #   jellyfin = {
  #     serviceConfig = {
  #       ReadWritePaths = [
  #         "/media/ssd1/Movies"
  #         "/media/ssd1/Shows"
  #       ];
  #     };
  #   };
  # };

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
