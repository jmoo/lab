{
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
    lab = {
      direnv.enable = true;
      vscode = {
        enable = true;
        nix.formatter = pkgs.nixfmt-rfc-style;
      };
    };

    programs.yt-dlp.enable = true;
  };

  lab = {
    source = "/home/jmoore/Repos/jmoo/lab";
    hyprland.enable = true;
    k3s.enable = true;
    shell.enable = true;
    ssh = {
      enable = true;
      users = [ "jmoore" ];
    };
    pass.enable = true;
  };

  networking.networkmanager = {
    enable = true;
  };

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
