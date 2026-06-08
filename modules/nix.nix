{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule;

  nix = {
    nix.settings.experimental-features = "nix-command flakes";
  };
in
{
  # Nix daemon settings, applied to every platform. nix-darwin needs
  # `nix.enable` to manage the daemon (NixOS enables it by default).
  options.lab.hosts = mkHostModule (_: {
    config = {
      nixos.module = nix;
      asahi.module = nix;
      darwin.module = {
        imports = [ nix ];
        nix.enable = true;
      };
    };
  });
}
