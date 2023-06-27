# export home-manager config as regular nix expressions for tinkering
# This file references files in my monorepo that this repo is a submodule of

with builtins;

{ nixpkgs ? <nixpkgs>,

pkgs ? import nixpkgs { },

home-manager ? let lock = fromJSON (readFile ../flake.lock);
in (fetchTarball (with lock.nodes.home-manager.locked; {
  url = "https://github.com/nix-community/home-manager/archive/${rev}.tar.gz";
  sha256 = lock.nodes.home-manager.locked.narHash;
})) }:

{
  nodes = pkgs.lib.mapAttrs (node: configuration:
    (import "${home-manager}/modules/default.nix" {
      inherit pkgs configuration;
    })) {
      home = ./nodes/home.nix;
      falcon = ./nodes/falcon.nix;
      lynx = ./nodes/lynx.nix;
      meerkat = ./nodes/meerkat/home.nix;
      mink = ./nodes/mink.nix;
    };
}
