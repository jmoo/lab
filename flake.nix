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
          { config, pkgs, ... }:
          let
            # One crate's corpus suite, run against a given corpus assembly.
            corpusSuite =
              corpus: crate:
              crate.overrideAttrs (old: {
                NORD_CORPUS_DIR = "${corpus}";
                cargoTestFlags = old.cargoTestFlags ++ [
                  "--features"
                  "corpus"
                ];
                doCheck = true;
              });
          in
          {
            checks = {
              # Test nord-format against real clavia nord files.
              # This requires access to a private repository (jmoo/nord-corpus)
              nord-format-corpus = corpusSuite pkgs.nord-corpus pkgs.nord-format;

              # Test nord-usb's wire, session and envelope layers against the captured
              # NSM exchange shapes. Same private corpus as nord-format-corpus.
              nord-usb-corpus = corpusSuite pkgs.nord-corpus pkgs.nord-usb;
            };

            packages = {
              inherit (pkgs)
                anki-tool
                nord-cli
                nord-corpus-full
                nord-format
                nord-usb
                open-bamboo-networking
                ;

              # The everything-run: both suites against the corpus with its R2 tier
              # spliced in, so the sweeps see the bundle archives, the untrimmed captures
              # and the whole vendor sample pool.
              #
              # ⚠️ Packages, not checks: these need a store seeded by `corpus nix-add`
              # (or R2 credentials in the builder), and `nix flake check` has to stay
              # runnable with neither. See `docs/nord-corpus.md`.
              corpus-full = pkgs.linkFarm "nord-corpus-full-suites" {
                nord-format = config.packages.nord-format-corpus-full;
                nord-usb = config.packages.nord-usb-corpus-full;
              };

              nord-format-corpus-full = corpusSuite pkgs.nord-corpus-full pkgs.nord-format;

              nord-usb-corpus-full = corpusSuite pkgs.nord-corpus-full pkgs.nord-usb;
            };
          };
      };
}
