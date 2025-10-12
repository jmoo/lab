{
  pkgs,
  lib,
  config,
  ...
}:
{
  environment.systemPackages = with pkgs; [ tailscale ];

  home-manager.users.jmoore = {
    home = {
      packages = with pkgs; [
        binwalk
        colordiff
        radare2
        vbindiff
      ];

      shellAliases = {
        "nix-lynx" = "nix --store 'ssh-ng://lynx.johndm.dev'";
      };
    };
    programs.yt-dlp.enable = true;
  };

  lab = {
    source = "${config.home-manager.users.jmoore.home.homeDirectory}/Repos/jmoo/lab";
    users = [ "jmoore" ];
    root = true;

    direnv = {
      enable = true;
      root = true;
    };

    shell = {
      enable = true;
      root = true;
    };

    vscode = {
      enable = true;
      root = true;
      common.nix.formatter = pkgs.nixfmt-rfc-style;
    };
  };

  networking.hostName = "meerkat";

  nix = {
    distributedBuilds = true;

    buildMachines = [
      {
        hostName = "lynx.johndm.dev";
        sshKey = "${config.home-manager.users.jmoore.home.homeDirectory}/.ssh/id_rsa";
        sshUser = "nix-ssh";
        maxJobs = 8;
        supportedFeatures = [
          "kvm"
          "big-parallel"
        ];
        systems = [
          "aarch64-linux"
          "x86_64-linux"
        ];
      }
    ];

    extraOptions = ''
      extra-platforms = aarch64-linux
      builders-use-substitutes = true
    '';

    settings = {
      trusted-users = [
        "jmoore"
        "root"
      ];

      trusted-substituters = [
        "ssh://lynx.johndm.dev"
      ];

      trusted-public-keys = [
        "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
        (builtins.readFile ../lynx/pubkeys/nix.pub)
      ];

      substituters = [
        "https://cache.nixos.org/"
      ];
    };
  };
}
