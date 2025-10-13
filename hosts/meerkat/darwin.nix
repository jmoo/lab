{
  pkgs,
  lib,
  ...
}:
with lib;
{
  imports = [
    ../../modules/darwin.nix
    ./common.nix
  ];

  environment.systemPackages = with pkgs; [ tailscale ];

  lab = {
    iterm2.enable = true;
  };

  users.users.jmoore = {
    name = "jmoore";
    home = "/Users/jmoore";
  };
}
