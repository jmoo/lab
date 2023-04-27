# export home-manager config as regular nix expressions for tinkering
# this repo lives as a submodule in a private git repo I use that has all of my projects on one place
# that explains the relative paths to nowhere

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
      macaw = ./nodes/macaw.nix;
      meerkat = ./nodes/meerkat.nix;
      mink = ./nodes/mink.nix;
    };
}
