{
  nixpkgs,
  nix-darwin,
  self,
  ...
}@inputs:
rec {
  aliasBuilds = x: x.config.system.build // x;

  eachSystem = nixpkgs.lib.genAttrs [
    "aarch64-darwin"
    "aarch64-linux"
    "x86_64-linux"
  ];

  eachPackageSet = f: nixpkgs.lib.mapAttrs (_: f) self.legacyPackages;

  darwinSystem =
    module:
    aliasBuilds (
      nix-darwin.lib.darwinSystem {
        system = "aarch64-darwin";
        specialArgs = {
          inherit inputs;
        };
        modules = [
          { nixpkgs.overlays = nixpkgs.lib.attrValues self.overlays; }
          module
        ];
      }
    );

  nixosSystem =
    system: module:
    aliasBuilds (
      nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = {
          inherit inputs;
        };
        modules = [
          { nixpkgs.overlays = nixpkgs.lib.attrValues self.overlays; }
          module
        ];
      }
    );
}
