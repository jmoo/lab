{
  lib',
  config,
  inputs,
  ...
}:
let
  inherit (lib'.lab) mkHostModule forAll;
  inherit (lib') mkOption types;
in
{
  options = {
    lab.hosts = mkHostModule (forAll {
      inherit (config) nixpkgs;
    });

    nixpkgs = {
      config = mkOption {
        default = { };
        type = types.attrsOf types.anything;
      };

      overlays = mkOption {
        default = [ ];
        type = types.listOf types.raw;
      };
    };
  };

  config = {
    nixpkgs = {
      config = {
        allowUnfree = true;
        # orca-slicer depends on libsoup 2
        permittedInsecurePackages = [
          "libsoup-2.74.3"
        ];
      };
      overlays = [ (import ../overlay.nix inputs) ];
    };

    perSystem =
      { system, pkgs, ... }:
      {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          inherit (config.nixpkgs) config overlays;
        };

        legacyPackages = pkgs;
      };

    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "aarch64-darwin"
    ];
  };
}
