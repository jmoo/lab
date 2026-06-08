{
  flake-parts,
  import-tree,
  home-manager,
  ...
}@inputs:
final: prev:
let
  inherit (final) mkOption mkOptionDefault types;
in
{
  mkFlake =
    args: module:
    let
      args' = args // {
        inputs = inputs // args.inputs;
      };
    in
    flake-parts.lib.mkFlake args' (_: {
      imports = [
        (import-tree ./new-modules)
        (import-tree ./new-hosts)
        module
      ];

      _module.args.lib' = final;
    });

  lab = {
    mkHostOptions =
      options:
      mkOption {
        type =
          with types;
          attrsOf (
            (submodule (_: {
              inherit options;
            }))
          );
      };

    mkHostPlatform =
      module:
      mkOption {
        type =
          with types;
          submodule (
            { config, ... }:
            {
              imports = [ module ];
              options = {
                enable = mkOption {
                  type = types.bool;
                };
                eval = mkOption {
                  type = types.bool;
                };
                module = mkOption {
                  type = types.deferredModule;
                };
              };

              config = {
                enable = mkOptionDefault false;
                eval = mkOptionDefault config.enable;
              };
            }
          );
      };
  };
}
