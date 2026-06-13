{
  lib',
  inputs,
  config,
  ...
}:
let
  inherit (lib'.lab) mkHostOptions mkHostPlatform;
  inherit (lib')
    filterAttrs
    mapAttrs
    mkDefault
    mkOption
    nixosSystem
    types
    ;

  # Apply this platform's home-manager configuration to the host's user.
  homeManager = host: {
    home-manager = {
      useGlobalPkgs = true;
      useUserPackages = true;
      users.${host.user} =
        { osConfig, ... }:
        {
          home.stateVersion = mkDefault osConfig.system.stateVersion;
          imports = [
            host.home
            host.nixos.home
          ];
          programs.home-manager.enable = false;
        };
    };
  };
in
{
  options = {
    lab.hosts = mkHostOptions {
      nixos = mkHostPlatform {
        config = {
          module = {
            imports = [
              inputs.home-manager.nixosModules.home-manager
            ];

            # stateVersion is stateful — pin to the hosts' install version, not
            # the current nixpkgs release. Override per host if newer.
            # Locale, nix, and nixpkgs config come from the shared feature
            # modules (locale.nix / nix.nix / nixpkgs.nix).
            system.stateVersion = mkDefault "25.05";
          };
        };

        options = {
          home = mkOption {
            default = { };
            description = "Home-manager configuration for this platform's user";
            type = types.deferredModule;
          };

          specialArgs = mkOption {
            default = { };
            type = types.attrsOf types.anything;
          };

          system = mkOption {
            type = types.str;
          };
        };
      };
    };
  };

  config = {
    flake = {
      nixosConfigurations = mapAttrs (
        _: host:
        nixosSystem {
          modules = [
            host.nixos.module
            (homeManager host)
          ];
          specialArgs = host.nixos.specialArgs;
          system = host.nixos.system;
        }
      ) (filterAttrs (_: host: host.nixos.eval) config.lab.hosts);

      nixosModules = mapAttrs (_: host: {
        imports = [
          host.nixos.module
          (homeManager host)
        ];
      }) (filterAttrs (_: host: host.nixos.enable) config.lab.hosts);
    };
  };
}
