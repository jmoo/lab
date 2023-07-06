{ config, pkgs, lib, ... }:

with builtins;
with lib;

let youtube-dl = import ../packages/youtube-dl.nix { inherit pkgs; };
in {
  options.lab.youtube = { enable = mkEnableOption "youtube"; };

  config.home = mkIf config.lab.youtube.enable { packages = [ youtube-dl ]; };
}
