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

    forLinux = module: {
      asahi.module = module;
      nixos.module = module;
    };

    homeDarwin = module: {
      darwin.home = module;
    };

    homeLinux = module: {
      asahi.home = module;
      nixos.home = module;
    };

    mkScript =
      pkgs: scriptsDir: filename:
      let
        match = builtins.match "(.+)\\.[^.]+" filename;
        name = if match != null then builtins.head match else filename;
        src = builtins.readFile (scriptsDir + "/${filename}");
        lines = final.splitString "\n" src;

        secondLine = if builtins.length lines > 1 then builtins.elemAt lines 1 else "";
        depsLine = if final.hasPrefix "# nix-deps: " secondLine then secondLine else null;
        runtimeInputs =
          if depsLine != null then
            map (d: pkgs.${d}) (
              builtins.filter (s: s != "") (final.splitString " " (final.removePrefix "# nix-deps: " depsLine))
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
