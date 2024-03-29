# home-manager switch --flake ./lab/nodes/meerkat
{
  description = "jmoo/lab";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-23.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    (flake-utils.lib.eachDefaultSystem (system:
      let pkgs = import nixpkgs { inherit system; };
      in {
        devShells.default =
          pkgs.mkShell { packages = with pkgs; [ nil nixfmt ]; };
      })
    ) // {
      lib.home = import ./home.nix;
    };
}
