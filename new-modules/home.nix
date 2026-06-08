{ lib', config, ... }:
let
  inherit (lib'.lab) mkHostOptions mkHostPlatform;
  inherit (lib') mkDefault mapAttrs filterAttrs;
in
{
  options = {
    lab = {
      hosts = mkHostOptions {
        home = mkHostPlatform {
          eval = mkDefault false;
          module = { };
        };
      };
    };
  };

 config = {
    flake = {
      homeModules = mapAttrs (_: host: host.home.module) (
        filterAttrs (_: host: host.home.enable) config.lab.hosts
      );
    };
  };
}
