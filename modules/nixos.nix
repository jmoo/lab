{
  lib',
  inputs,
  config,
  ...
}:
let
  inherit (lib'.lab) mkHostOptions mkHostPlatform;
  inherit (lib')
    mkOption
    mkDefault
    mapAttrs
    filterAttrs
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
          imports = [
            host.home
            host.nixos.home
          ];
          home.stateVersion = mkDefault osConfig.system.stateVersion;
          programs.home-manager.enable = false;
        };
    };
  };
in
{
  options = {
    lab = {
      hosts = mkHostOptions {
        nixos = mkHostPlatform {
          options = {
            specialArgs = mkOption {
              type = types.attrsOf types.anything;
              default = { };
            };

            system = mkOption {
              type = types.str;
            };

            home = mkOption {
              description = "Home-manager configuration for this platform's user";
              type = types.deferredModule;
              default = { };
            };
          };

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
        };
      };
    };
  };

  config = {
    flake = {
      nixosConfigurations = mapAttrs (
        _: host:
        nixosSystem {
          system = host.nixos.system;
          specialArgs = host.nixos.specialArgs;
          modules = [
            host.nixos.module
            (homeManager host)
          ];
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
