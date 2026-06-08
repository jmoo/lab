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
