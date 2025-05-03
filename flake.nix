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

    nix-environments.url = "github:nix-community/nix-environments";

    nixos-xlnx = {
      url = "github:chuangzhu/nixos-xlnx";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, self, ... }@inputs:
    let
      inherit (import ./lib.nix inputs)
        eachPackageSet
        eachSystem
        darwinSystem
        nixosSystem
        ;
    in
    {
      legacyPackages = eachSystem (
        system:
        import nixpkgs {
          inherit system;
          overlays = nixpkgs.lib.attrValues self.overlays;
        }
      );

      overlays.default = import ./overlay.nix inputs;

      darwinConfigurations = {
        meerkat = darwinSystem ./hosts/meerkat;
      };

      devShells = eachPackageSet (pkgs: {
        zebu = pkgs.callPackage ./hosts/zebu/shell.nix {
          inherit inputs;
        };
      });

      formatter = eachPackageSet (pkgs: pkgs.nixfmt-rfc-style);

      nixosConfigurations = {
        lynx = nixosSystem "x86_64-linux" ./hosts/lynx;
        zebu = nixosSystem "aarch64-linux" ./hosts/zebu;
      };

      nixosModules = {
        axolotl = import ./hosts/axolotl;
        darwin = import ./modules/darwin.nix;
        home = import ./modules/home.nix;
        lynx = import ./hosts/lynx;
        nixos = import ./modules/nixos.nix;
        zebu = import ./hosts/zebu;
      };
    };
}
