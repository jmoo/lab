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

    nixos-apple-silicon = {
      url = "github:nix-community/nixos-apple-silicon?ref=release-2025-08-23";
    };

    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, ... }@inputs:
    rec {
      darwinConfigurations = {
        meerkat = inputs.nix-darwin.lib.darwinSystem {
          system = "aarch64-darwin";
          specialArgs = {
            inherit inputs;
          };
          modules = [
            ./hosts/meerkat/darwin.nix
            { nixpkgs.overlays = nixpkgs.lib.attrValues overlays; }
          ];
        };
      };

      formatter = nixpkgs.lib.mapAttrs (_: pkgs: pkgs.nixfmt-rfc-style) legacyPackages;

      nixosConfigurations = {
        lynx = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          specialArgs = {
            inherit inputs;
          };
          modules = [
            ./hosts/lynx
            { nixpkgs.overlays = nixpkgs.lib.attrValues overlays; }
          ];
        };

        meerkat = nixpkgs.lib.nixosSystem {
          system = "aarch64-linux";
          specialArgs = {
            inherit inputs;
          };
          modules = [
            ./hosts/meerkat/asahi.nix
            { nixpkgs.overlays = nixpkgs.lib.attrValues overlays; }
          ];
        };
      };

      nixosModules = {
        axolotl = import ./hosts/axolotl/default.nix;
        lynx = import ./hosts/lynx/default.nix;
        meerkat-asahi = import ./hosts/meerkat/asahi.nix;
        meerkat-darwin = import ./hosts/meerkat/darwin.nix;
        default = import ./modules/nixos.nix;
      };

      overlays.default = import ./overlay.nix inputs;

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
