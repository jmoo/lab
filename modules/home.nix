{
  lib',
  inputs,
  config,
  ...
}:
let
  inherit (lib')
    filterAttrs
    mapAttrs
    mkDefault
    ;
in
{
  config.flake.homeConfigurations = mapAttrs (
    _: host:
    inputs.home-manager.lib.homeManagerConfiguration {
      pkgs = import inputs.nixpkgs {
        inherit (config.nixpkgs) config overlays;
        system = host.home.system;
      };
      modules = [
        host.home.module
        {
          home = {
            homeDirectory = mkDefault "/home/${host.user}";
            shellAliases.switch = mkDefault "home-manager switch --flake ${host.source}#${host.name}";
            stateVersion = mkDefault "25.05";
            username = mkDefault host.user;
          };
          programs.home-manager.enable = true;
        }
      ];
    }
  ) (filterAttrs (_: host: host.home.eval) config.lab.hosts);
}
