{ lib', inputs, ... }:
let
  inherit (inputs) home-manager;
  inherit (lib') mkOption types;
in
{
  imports = [
    home-manager.flakeModules.home-manager
  ];

  options = {
    lab = {
      hosts = mkOption {
        type =
          with types;
          attrsOf (
            submodule (
              { config, name, ... }:
              {
                options = {
                  name = mkOption {
                    type = types.str;
                    default = name;
                  };

                  user = mkOption {
                    description = "The single user home-manager configuration is applied to";
                    type = types.str;
                  };

                  source = mkOption {
                    description = "Flake uri of the jmoo/lab source, used by the rebuild alias";
                    type = with types; nullOr str;
                    default = "github:jmoo/lab";
                  };
                };

                config = {
                  _module.args.host = config;
                };
              }
            )
          );
        default = { };
      };
    };
  };

  config = {
    debug = true;
  };
}
