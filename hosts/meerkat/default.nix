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
    home.packages = with pkgs; [ spotify ];
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
      nix.formatter = pkgs.nixfmt-rfc-style;
    };
  };

  networking.hostName = "meerkat";

  users.users.jmoore = {
    name = "jmoore";
    home = "/Users/jmoore";
  };
}
