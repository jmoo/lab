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

    mkScripts =
      pkgs: dir:
      let
        entries = builtins.readDir dir;
        files = builtins.filter (name: entries.${name} == "regular") (builtins.attrNames entries);
        mkName =
          filename:
          let
            m = builtins.match "(.+)\\.[^.]+" filename;
          in
          if m != null then builtins.head m else filename;
      in
      builtins.listToAttrs (
        map (filename: {
          name = mkName filename;
          value = final.lab.mkScript pkgs dir filename;
        }) files
      );

    mkScript =
      pkgs: scriptsDir: filename:
      let
        match = builtins.match "(.+)\\.[^.]+" filename;
        name = if match != null then builtins.head match else filename;
        src = builtins.readFile (scriptsDir + "/${filename}");
        lines = final.splitString "\n" src;

        secondLine = if builtins.length lines > 1 then builtins.elemAt lines 1 else "";
        depsLine = if final.hasPrefix "# nix-deps: " secondLine then secondLine else null;
        depNames =
          if depsLine != null then
            builtins.filter (s: s != "") (final.splitString " " (final.removePrefix "# nix-deps: " depsLine))
          else
            [ ];
        depsAttrs = builtins.listToAttrs (
          map (d: {
            name = d;
            value = pkgs.${d};
          }) depNames
        );

        # Strip shebang so writeShellApplication can supply its own.
        body = final.concatStringsSep "\n" (
          if lines != [ ] && final.hasPrefix "#!" (builtins.head lines) then builtins.tail lines else lines
        );
      in
      pkgs.callPackage (
        { writeShellApplication, ... }@args:
        writeShellApplication {
          inherit name;
          runtimeInputs = map (d: args.${d}) depNames;
          text = body;
        }
      ) depsAttrs;

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
