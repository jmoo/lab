{
  lib',
  config,
  inputs,
  ...
}:
let
  inherit (lib'.lab) mkHostModule forAll;
  inherit (lib') types mkOption;
in
{
  options = {
    lab = {
      hosts = mkHostModule (forAll {
        inherit (config) nixpkgs;
      });
    };

    nixpkgs = {
      overlays = mkOption {
        type = types.listOf types.raw;
        default = [ ];
      };

      config = mkOption {
        type = types.attrsOf types.anything;
        default = { };
      };
    };
  };

  config = {
    nixpkgs = {
      config = {
        allowUnfree = true;
        permittedInsecurePackages = [
          "libsoup-2.74.3"
        ];
      };
      overlays = [ (import ../overlay.nix inputs) ];
    };

    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "aarch64-darwin"
    ];

    perSystem =
      { system, pkgs, ... }:
      {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          inherit (config.nixpkgs) config overlays;
        };

        legacyPackages = pkgs;
      };
  };
}
