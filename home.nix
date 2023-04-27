# base config shared by all machines

{ config, pkgs, lib, ... }:

with lib;
with builtins;

{
  imports = [
    ./modules/cinnamon.nix
    ./modules/karabiner.nix
    ./modules/shell.nix
    ./modules/guake.nix
    ./modules/nuphy75.nix
    ./modules/astronvim.nix
  ];

  lab.shell.enable = mkDefault true;

  home.stateVersion = mkDefault "22.11";

  # Packages
  home.packages = with pkgs; [ jq yq nixfmt ];

  programs.home-manager.enable = mkDefault true;

  home.sessionVariables = { HOMELAB_ROOT = "${builtins.toString ./..}"; };

  home.username = mkDefault "jmoore";

  home.homeDirectory = mkDefault
    (if pkgs.stdenv.isDarwin then "/Users/jmoore" else "/home/jmoore");
}
