# arm macbook pro

{ pkgs, lib, ... }:

with lib;
with builtins;

{
  imports = [ ../../home.nix ];

  home.username = "deck";
  home.homeDirectory = "/home/deck";
  home.stateVersion = "23.05";

  home.packages = with pkgs; [
    tailscale
    vim
  ];

  lab.shell.init = ''
    switch() {
      home-manager switch --flake /home/deck/Repos/homelab/lab/nodes/deck/flake.nix
    }
  '';
}
