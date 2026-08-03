{
  inputs = {
    # Rust toolchains with per-target std libraries. nixpkgs' rustc ships only the
    # host target plus wasm32, which is not enough to cross-compile nord-usb to
    # Windows/Linux. fenix exposes packages per-system, so this stays scoped to the
    # Rust builds and never touches any host's overlay set.
    fenix = {
      inputs.nixpkgs.follows = "nixpkgs";
      url = "github:nix-community/fenix";
    };

    flake-parts.url = "github:hercules-ci/flake-parts";

    home-manager = {
      inputs.nixpkgs.follows = "nixpkgs";
      url = "github:nix-community/home-manager";
    };

    # Asahi uses the pinned nixos-apple-silicon nixpkgs (25.11, Aug 2025), whose
    # lib predates home-manager master's use of `lib.genAttrs'`. Pin a matching
    # home-manager from the same era for the asahi platform only.
    home-manager-asahi = {
      inputs.nixpkgs.follows = "nixos-apple-silicon/nixpkgs";
      url = "github:nix-community/home-manager/dd026d864207";
    };

    import-tree.url = "github:denful/import-tree";

    nix-darwin = {
      inputs.nixpkgs.follows = "nixpkgs";
      url = "github:lnl7/nix-darwin";
    };

    nixos-apple-silicon = {
      url = "github:nix-community/nixos-apple-silicon?ref=release-2025-08-23";
      # Kernel panic on unstable, use nixos-apple-silicon's nixpkgs pin
      # inputs.nixpkgs.follows = "nixpkgs";
    };

    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    nudelta.url = "github:donn/nudelta";

    treefmt-nix = {
      inputs.nixpkgs.follows = "nixpkgs";
      url = "github:numtide/treefmt-nix";
    };
  };

  outputs =
    { nixpkgs, ... }@inputs:
    (nixpkgs.lib.extend (import ./lib.nix inputs)).mkFlake
      {
        inherit inputs;
      }
      {
        perSystem =
          { lib, pkgs, ... }:
          {
            checks = {
              # The specimen expectations are only pinned if the specimens are: the
              # corpus revision the suite is blessed against has to be the one the
              # overlay fetches, or a green Nix check and a green local run are checking
              # different files.
              corpus-rev-agrees =
                let
                  blessed = lib.trim (builtins.readFile ./crates/nord-format/tests/corpus_rev.txt);
                in
                pkgs.runCommand "corpus-rev-agrees" { } ''
                  if [ "${blessed}" != "${pkgs.nord-corpus-rev}" ]; then
                    echo "overlay.nix pins nord-corpus at ${pkgs.nord-corpus-rev}," >&2
                    echo "but tests/corpus_rev.txt blesses ${blessed} — move one to the other." >&2
                    exit 1
                  fi
                  touch $out
                '';

              # Test nord-format against real clavia nord files.
              # This requires access to a private repository (jmoo/nord-corpus)
              nord-format-corpus = pkgs.nord-format.overrideAttrs (old: {
                NORD_CORPUS_DIR = "${pkgs.nord-corpus}/ne5";
                cargoTestFlags = old.cargoTestFlags ++ [
                  "--features"
                  "corpus"
                ];
                doCheck = true;
              });

              # Test nord-usb's wire, session and envelope layers against the captured
              # NSM exchange shapes. Same private corpus as nord-format-corpus.
              nord-usb-corpus = pkgs.nord-usb.overrideAttrs (old: {
                NORD_CORPUS_DIR = "${pkgs.nord-corpus}/ne5";
                cargoTestFlags = old.cargoTestFlags ++ [
                  "--features"
                  "corpus"
                ];
                doCheck = true;
              });
            };

            packages = {
              inherit (pkgs)
                anki-tool
                nord-cli
                nord-format
                nord-usb
                open-bamboo-networking
                ;
            };
          };
      };
}
