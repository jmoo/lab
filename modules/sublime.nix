{ config, pkgs, lib, ... }:

with builtins;
with lib;

let
  sublime = import ../packages/sublime4.nix { inherit pkgs; };
in
{
  options.lab.sublime = {
    enable = mkEnableOption "sublime";
  };

  config.home = mkIf config.lab.sublime.enable {
    packages = [
      sublime
    ];
  };
}