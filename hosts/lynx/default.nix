{ pkgs, ... }:
{
  imports = [
    ../../modules/nixos.nix
    ./hardware.nix
  ];

  environment.systemPackages = with pkgs; [
    git
    wireshark
    vim
  ];

  home-manager = {
    useGlobalPkgs = true;
    useUserPackages = true;
    users.jmoore = _: {
      imports = [ ../../modules/home.nix ];

      home = {
        homeDirectory = "/home/jmoore";
        username = "jmoore";
        shellAliases = {
          switch = "nixos-rebuild switch --flake /home/jmoore/Repos/jmoo/lab#lynx";
        };
      };

      lab = {
        direnv.enable = true;
        shell.enable = true;
        vscode = {
          enable = true;
          nix.formatter = pkgs.nixfmt-rfc-style;
        };
      };

      programs.yt-dlp.enable = true;
    };
  };

  programs.zsh.enable = true;

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

  # Enable sound with pipewire.
  hardware.pulseaudio.enable = false;
  security.rtkit.enable = true;
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
    # If you want to use JACK applications, uncomment this
    #jack.enable = true;

    # use the example session manager (no others are packaged yet so this is enabled by default,
    # no need to redefine it in your config for now)
    #media-session.enable = true;
  };

  # Configure network proxy if necessary
  # networking.proxy.default = "http://user:password@proxy:port/";
  # networking.proxy.noProxy = "127.0.0.1,localhost,internal.domain";

  # Enable networking
  networking.networkmanager.enable = true;

  # Set your time zone.
  time.timeZone = "America/New_York";

  # Select internationalisation properties.
  i18n.defaultLocale = "en_US.UTF-8";

  i18n.extraLocaleSettings = {
    LC_ADDRESS = "en_US.UTF-8";
    LC_IDENTIFICATION = "en_US.UTF-8";
    LC_MEASUREMENT = "en_US.UTF-8";
    LC_MONETARY = "en_US.UTF-8";
    LC_NAME = "en_US.UTF-8";
    LC_NUMERIC = "en_US.UTF-8";
    LC_PAPER = "en_US.UTF-8";
    LC_TELEPHONE = "en_US.UTF-8";
    LC_TIME = "en_US.UTF-8";
  };

  # Enable the X11 windowing system.
  services.xserver.enable = true;

  # Enable the Cinnamon Desktop Environment.
  services.xserver.displayManager.lightdm.enable = true;
  services.xserver.desktopManager.cinnamon.enable = true;

  # Configure keymap in X11
  services.xserver.xkb = {
    layout = "us";
    variant = "";
  };

  # Enable CUPS to print documents.
  services.printing.enable = true;
}
