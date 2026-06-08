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
          imports = [ host.nixos.home ];
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

              nix.settings.experimental-features = "nix-command flakes";

              nixpkgs = {
                config = {
                  allowUnfree = true;
                  permittedInsecurePackages = [
                    "libsoup-2.74.3"
                  ];
                };
                overlays = [ (import ../overlay.nix inputs) ];
              };

              # stateVersion is stateful — pin to the hosts' install version, not
              # the current nixpkgs release. Override per host if newer.
              system.stateVersion = mkDefault "25.05";
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
