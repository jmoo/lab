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
    types
    ;

  # Apply the host's single home-manager configuration to its user.
  homeManager = host: {
    home-manager = {
      useGlobalPkgs = true;
      useUserPackages = true;
      users.${host.user} = {
        home.stateVersion = mkDefault "25.05";
        imports = [
          host.home.module
          host.darwin.home
        ];
        programs.home-manager.enable = false;
      };
    };
  };
in
{
  options = {
    lab.hosts = mkHostOptions {
      darwin = mkHostPlatform {
        config = {
          module = {
            imports = [
              inputs.home-manager.darwinModules.home-manager
            ];

            # nix and nixpkgs.{config,overlays} come from the shared feature
            # modules (nix.nix / nixpkgs.nix).
            nixpkgs.hostPlatform = mkDefault "aarch64-darwin";

            system.stateVersion = mkDefault 5;

            users.users.root.home = "/var/root";
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
            default = "aarch64-darwin";
            type = types.str;
          };
        };
      };
    };
  };

  config = {
    flake = {
      darwinConfigurations = mapAttrs (
        _: host:
        inputs.nix-darwin.lib.darwinSystem {
          modules = [
            host.darwin.module
            (homeManager host)
          ];
          specialArgs = host.darwin.specialArgs;
          system = host.darwin.system;
        }
      ) (filterAttrs (_: host: host.darwin.eval) config.lab.hosts);

      darwinModules = mapAttrs (_: host: {
        imports = [
          host.darwin.module
          (homeManager host)
        ];
      }) (filterAttrs (_: host: host.darwin.enable) config.lab.hosts);
    };
  };
}
