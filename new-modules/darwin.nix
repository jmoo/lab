{
  lib',
  inputs,
  config,
  ...
}:
let
  inherit (lib'.lab) mkHostOptions mkHostPlatform;
  inherit (lib')
    mkDefault
    mkOption
    mapAttrs
    filterAttrs
    types
    ;

  # Apply the host's single home-manager configuration to its user.
  homeManager = host: {
    home-manager = {
      useGlobalPkgs = true;
      useUserPackages = true;
      users.${host.user} = {
        imports = [ host.darwin.home ];
        home.stateVersion = mkDefault "25.05";
        programs.home-manager.enable = false;
      };
    };
  };
in
{
  options = {
    lab = {
      hosts = mkHostOptions {
        darwin = mkHostPlatform {
          options = {
            specialArgs = mkOption {
              type = types.attrsOf types.anything;
              default = { };
            };

            system = mkOption {
              type = types.str;
              default = "aarch64-darwin";
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
                inputs.home-manager.darwinModules.home-manager
              ];

              users.users.root.home = "/var/root";

              nix.enable = true;
              nix.settings.experimental-features = "nix-command flakes";

              nixpkgs = {
                config.allowUnfree = true;
                overlays = [ (import ../overlay.nix inputs) ];
                hostPlatform = mkDefault "aarch64-darwin";
              };

              system.stateVersion = mkDefault 5;
            };
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
          system = host.darwin.system;
          specialArgs = host.darwin.specialArgs;
          modules = [
            host.darwin.module
            (homeManager host)
          ];
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
