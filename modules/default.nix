{ lib', inputs, ... }:
let
  inherit (inputs) home-manager;
  inherit (lib') mkOption types;
in
{
  config.debug = true;

  imports = [
    home-manager.flakeModules.home-manager
  ];

  options.lab.hosts = mkOption {
    type =
      with types;
      attrsOf (
        submodule (
          { config, name, ... }:
          {
            config._module.args.host = config;

            options = {
              home = mkOption {
                default = { };
                description = "Home-manager configuration applied to the user on every enabled platform";
                type = types.deferredModule;
              };

              name = mkOption {
                default = name;
                type = types.str;
              };

              source = mkOption {
                default = "github:jmoo/lab";
                description = "Flake uri of the jmoo/lab source, used by the rebuild alias";
                type = with types; nullOr str;
              };

              user = mkOption {
                description = "The single user home-manager configuration is applied to";
                type = types.str;
              };
            };
          }
        )
      );
    default = { };
  };
}
