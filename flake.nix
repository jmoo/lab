{
  inputs = {
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix-darwin = {
      url = "github:lnl7/nix-darwin";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { nixpkgs, ... }@inputs: rec {
    darwinConfigurations = {
      meerkat = inputs.nix-darwin.lib.darwinSystem {
        system = "aarch64-darwin";
        specialArgs = { inherit inputs; };
        modules = [
          ./hosts/meerkat
          { nixpkgs.overlays = nixpkgs.lib.attrValues overlays; }
        ];
      };
    };

    nixosModules = {
      axolotl = import ./hosts/axolotl/default.nix;
      default = import ./modules/nixos.nix;
    };

    overlays.default = import ./overlay.nix { inherit inputs; };

    legacyPackages = {
      aarch64-darwin = import nixpkgs {
        system = "aarch64-darwin";
        overlays = nixpkgs.lib.attrValues overlays;
      };

      x86_64-linux = import nixpkgs {
        system = "x86_64-linux";
        overlays = nixpkgs.lib.attrValues overlays;
      };
    };
  };
}
