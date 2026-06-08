{ lib', inputs, ... }:
let
  inherit (lib'.lab) mkHostModule;

  overlay = import ../overlay.nix inputs;

  # allowUnfree + the global overlay on every platform.
  common = {
    nixpkgs = {
      config.allowUnfree = true;
      overlays = [ overlay ];
    };
  };

  # Linux-only insecure-package allowances.
  linux = {
    nixpkgs.config.permittedInsecurePackages = [
      "libsoup-2.74.3"
    ];
  };
in
{
  options.lab.hosts = mkHostModule (_: {
    config = {
      nixos.module = {
        imports = [
          common
          linux
        ];
      };
      asahi.module = {
        imports = [
          common
          linux
        ];
      };
      darwin.module = common;
    };
  });
}
