{ config, pkgs, lib, ... }:

with builtins;
with lib;

{
  options.lab.sublime-merge = { 
    enable = mkEnableOption "sublime-merge"; 

	  package = mkOption {
	    type = types.package;
	    default = import ../packages/sublime-merge.nix { inherit pkgs; };
	  };
  };

  config.home = mkIf config.lab.sublime-merge.enable {
    packages = [ config.lab.sublime-merge.package ];
  };
}