{
  pkgs,
  config,
  ...
}:
{
  home-manager.users.jmoore = {
    home = {
      packages = with pkgs; [
        bat
        binwalk
        claude-code
        colordiff
        gh
        git
        radare2
        tio
        vbindiff
        vim
      ];
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

    greetd.enable = true;

    hyprpaper.enable = false;
    hyprland.enable = true;

    shell = {
      enable = true;
      root = true;
    };

    vscode = {
      enable = true;
      common.nix.formatter = pkgs.nixfmt-rfc-style;
    };
  };

  networking.hostName = "meerkat";

  nix = {
    distributedBuilds = true;

    extraOptions = ''
      extra-platforms = aarch64-linux
      builders-use-substitutes = true
    '';

    settings = {
      trusted-users = [
        "jmoore"
        "root"
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
