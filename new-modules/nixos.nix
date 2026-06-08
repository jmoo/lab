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
          };

          config = {
            eval = mkDefault true;

            module = {
              imports = [
                inputs.home-manager.nixosModules.home-manager
              ];

              i18n = {
                defaultLocale = "en_US.UTF-8";
                extraLocaleSettings = {
                  LC_ADDRESS = "en_US.UTF-8";
                  LC_IDENTIFICATION = "en_US.UTF-8";
                  LC_MEASUREMENT = "en_US.UTF-8";
                  LC_MONETARY = "en_US.UTF-8";
                  LC_NAME = "en_US.UTF-8";
                  LC_NUMERIC = "en_US.UTF-8";
                  LC_PAPER = "en_US.UTF-8";
                  LC_TELEPHONE = "en_US.UTF-8";
                  LC_TIME = "en_US.UTF-8";
                };
              };

              nixpkgs = {
                config.permittedInsecurePackages = [
                  "libsoup-2.74.3"
                ];
              };

              system.stateVersion = mkDefault "26.11";
              time.timeZone = "America/New_York";
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
          modules = [ host.nixos.module ];
        }
      ) (filterAttrs (_: host: host.nixos.eval) config.lab.hosts);

      nixosModules = mapAttrs (_: host: host.nixos.module) (
        filterAttrs (_: host: host.nixos.enable) config.lab.hosts
      );
    };
  };
}
