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

    # Like mkHostOptions but the argument is a full module (options *and*
    # config), letting a feature module both declare host-level options and
    # push config down into the per-platform `module` deferredModules.
    mkHostModule =
      module:
      mkOption {
        type = with types; attrsOf (submodule module);
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

    # Push a system module down into every Linux platform / every platform.
    forLinux = module: {
      nixos.module = module;
      asahi.module = module;
    };

    forAll = module: {
      nixos.module = module;
      asahi.module = module;
      darwin.module = module;
    };

    # Push a home-manager module down into each platform's home config.
    homeLinux = module: {
      nixos.home = module;
      asahi.home = module;
    };

    homeAll = module: {
      nixos.home = module;
      asahi.home = module;
      darwin.home = module;
    };

    homeDarwin = module: {
      darwin.home = module;
    };
  };
}
