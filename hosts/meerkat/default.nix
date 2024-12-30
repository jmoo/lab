{
  pkgs,
  lib,
  mkHome,
  ...
}:
with lib;
{
  imports = [ ../../modules/darwin.nix ];

  environment.systemPackages = with pkgs; [ tailscale ];

  home-manager.users.jmoore = mkHome {
    lab = {
      direnv.enable = true;
      iterm2.enable = true;
      vscode = {
        enable = true;
        nix.formatter = pkgs.nixfmt-rfc-style;
      };
    };

    home.packages = with pkgs; [ spotify ];

    programs.yt-dlp.enable = true;
  };

  lab = {
    source = "/Users/jmoore/Repos/jmoo/lab";
    shell.enable = true;
  };

  networking.hostName = "meerkat";

  users.users.jmoore = {
    name = "jmoore";
    home = "/Users/jmoore";
  };
}
