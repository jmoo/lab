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
          let
            # The full golden specimen corpus lives in the private
            # jmoo/nord-corpus repo (it grows to hold proprietary piano/sample
            # data). Fetched lazily over SSH using the caller's key — this thunk
            # is only forced when the `nord-format-corpus` check is built, so a
            # plain `nix build .#nord-format` never fetches or needs it.
            nord-corpus = builtins.fetchGit {
              rev = "b80431bcccddbb07bf5bcccb7dce42968c404898";
              url = "git+ssh://git@github.com/jmoo/nord-corpus.git";
            };
          in
          {
            # "With tests" target: rebuilds nord-format and runs the full
            # corpus-backed round-trip sweep against the fetched specimens.
            # `nix flake check` runs it; needs access to the private corpus.
            checks.nord-format-corpus = pkgs.nord-format.overrideAttrs (_: {
              NORD_CORPUS_DIR = "${nord-corpus}/ne5";
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
