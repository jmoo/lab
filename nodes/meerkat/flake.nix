# home-manager switch --flake ./lab/nodes/meerkat
{
  description = "meerkat";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-23.05";

    home-manager = {
      url = "github:nix-community/home-manager/release-23.05";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, home-manager }:
    let
      pkgs = import nixpkgs {
        system = "aarch64-darwin";
        config = { allowUnfree = true; };
      };
    in {
      homeConfigurations.jmoore = home-manager.lib.homeManagerConfiguration {
        inherit pkgs;

        modules = [ ./home.nix ];
      };

      devShells.aarch64-darwin.default = pkgs.mkShell {
        packages = with pkgs; [ nil nixfmt ];
      };
    };
}
