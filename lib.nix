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
  lab = {
    forAll = module: {
      asahi.module = module;
      darwin.module = module;
      nixos.module = module;
    };

    # Push a system module down into every Linux platform / every platform.
    forLinux = module: {
      asahi.module = module;
      nixos.module = module;
    };

    homeDarwin = module: {
      darwin.home = module;
    };

    # Push a home-manager module down into each platform's home config.
    # (For all platforms, set the host-level `home` option instead.)
    homeLinux = module: {
      asahi.home = module;
      nixos.home = module;
    };

    # Like mkHostOptions but the argument is a full module (options *and*
    # config), letting a feature module both declare host-level options and
    # push config down into the per-platform `module` deferredModules.
    mkScript =
      pkgs: scriptsDir: filename:
      let
        name = final.removeSuffix ".sh" filename;
        src = builtins.readFile (scriptsDir + "/${filename}");
        lines = final.splitString "\n" src;

        depsLine = final.findFirst (l: final.hasPrefix "# deps:" l) null lines;
        runtimeInputs =
          if depsLine != null then
            map (d: pkgs.${d}) (
              builtins.filter (s: s != "") (final.splitString " " (final.removePrefix "# deps: " depsLine))
            )
          else
            [ ];

        # Strip shebang so writeShellApplication can supply its own.
        body = final.concatStringsSep "\n" (
          if lines != [ ] && final.hasPrefix "#!" (builtins.head lines) then builtins.tail lines else lines
        );
      in
      pkgs.writeShellApplication {
        inherit name runtimeInputs;
        text = body;
      };

    mkHostModule =
      module:
      mkOption {
        type = with types; attrsOf (submodule module);
      };

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

              config = {
                enable = mkOptionDefault false;
                eval = mkOptionDefault config.enable;
              };

              options = {
                enable = mkOption { type = types.bool; };
                eval = mkOption { type = types.bool; };
                module = mkOption { type = types.deferredModule; };
              };
            }
          );
      };
  };

  mkFlake =
    args: module:
    let
      args' = args // {
        inputs = inputs // args.inputs;
      };
    in
    flake-parts.lib.mkFlake args' (_: {
      imports = [
        (import-tree ./modules)
        (import-tree ./hosts)
        module
      ];

      flake.lib = final;
      _module.args.lib' = final;
    });
}
