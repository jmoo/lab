{
  inputs = {
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
  };

  outputs =
    { nixpkgs, ... }@inputs:
    (nixpkgs.lib.extend (import ./lib.nix inputs)).mkFlake
      {
        inherit inputs;
      }
      {
        perSystem =
          { pkgs, ... }:
          {
            # Test nord-format against real clavia nord files.
            # This requires access to a private repository (jmoo/nord-corpus).
            # The corpus-backed tests are gated behind the `corpus` feature.
            checks.nord-format-corpus = pkgs.nord-format.overrideAttrs (old: {
              NORD_CORPUS_DIR = "${pkgs.nord-corpus}/ne5";
              cargoTestFlags = old.cargoTestFlags ++ [
                "--features"
                "corpus"
              ];
              doCheck = true;
            });

            packages = {
              inherit (pkgs)
                anki-tool
                nord-cli
                nord-format
                open-bamboo-networking
                ;
            };
          };
      };
}
