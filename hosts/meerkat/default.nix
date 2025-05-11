{
  pkgs,
  lib,
  ...
}:
with lib;
{
  imports = [ ../../modules/darwin.nix ];

  environment.systemPackages = with pkgs; [ tailscale ];

  home-manager.users.jmoore = {
    home.packages = with pkgs; [
      spotify
      binwalk
      colordiff
      radare2
      vbindiff
    ];

    lab.iterm2.enable = true;
    programs.yt-dlp.enable = true;
  };

  lab = {
    source = "/Users/jmoore/Repos/jmoo/lab";
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
    extraOptions = ''
      builders = @/etc/nix/machines
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
        "ssh://lynx.johndm.dev"
      ];
    };
  };

  users.users.jmoore = {
    name = "jmoore";
    home = "/Users/jmoore";
  };
}
