{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-22.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; overlays = [ rust-overlay.overlays.default ]; };

        rust-bin = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
          extensions = [ "rust-src" ];
        });
      in {
        packages.default = import ./default.nix { inherit pkgs; };
        
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            rust-bin
            rust-analyzer
            nil
            nixfmt
            python3
            nodejs
          ];
        };
      }
    );
}