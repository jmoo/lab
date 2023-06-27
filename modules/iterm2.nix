{ config, pkgs, lib, ... }:

with builtins;
with lib;

{
  options.lab.iterm2 = {
    enable = mkEnableOption "iterm2";
  };

  config.home = mkIf config.lab.iterm2.enable {
    packages = with pkgs; [
      iterm2
    ];

    file.iterm2-plist = {
      executable = false;
      source = ../dotfiles/iterm2.plist;
      target = ".config/iterm2/com.googlecode.iterm2.plist";
    };
  };
}