{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nix-darwin.url = "github:lnl7/nix-darwin";
    nix-darwin.inputs.nixpkgs.follows = "nixpkgs";
    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";
    lab.url = "github:jmoo/lab";
    lab.flake = false;
  };

  outputs = { nixpkgs, nix-darwin, home-manager, lab, ... }@inputs: rec {
    darwinConfigurations = {
      meerkat = nix-darwin.lib.darwinSystem {
        system = "aarch64-darwin";
        specialArgs = { inherit inputs; foo = "bar"; };
        modules = [ 
          ./hosts/meerkat
          { nixpkgs.overlays = nixpkgs.lib.attrValues overlays; }
        ];
      };
    };

    overlays.default = import ./overlay.nix { inherit inputs; };

    legacyPackages.aarch64-darwin = import nixpkgs {
      system = "aarch64-darwin";
      overlays = nixpkgs.lib.attrValues overlays;
    };
  };
}
