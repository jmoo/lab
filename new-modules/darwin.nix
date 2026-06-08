{ lib', config, ... }:
let
  inherit (lib'.lab) mkHostOptions mkHostPlatform;
  inherit (lib') mkDefault mapAttrs filterAttrs;
in
{
  options = {
    lab = {
      hosts = mkHostOptions {
        darwin = mkHostPlatform {
          eval = mkDefault true;

          module = {
            users.users.root.home = "/var/root";
            nix.enable = true;
            system.stateVersion = mkDefault 5;
            nixpkgs.hostPlatform = mkDefault "aarch64-darwin";
          };
        };
      };
    };
  };

  config = {
    flake = {
      darwinModules = mapAttrs (_: host: host.darwin.module) (
        filterAttrs (_: host: host.darwin.enable) config.lab.hosts
      );
    };
  };
}
