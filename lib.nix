{
  flake-parts,
  import-tree,
  home-manager,
  ...
}@inputs:
final: prev:
let
  inherit (builtins)
    attrNames
    elemAt
    filter
    fromTOML
    head
    length
    listToAttrs
    match
    readDir
    readFile
    tail
    ;
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
        entries = readDir dir;
        files = filter (name: entries.${name} == "regular") (attrNames entries);
        mkName =
          filename:
          let
            m = match "(.+)\\.[^.]+" filename;
          in
          if m != null then head m else filename;
      in
      listToAttrs (
        map (filename: {
          name = mkName filename;
          value = final.lab.mkScript pkgs dir filename;
        }) files
      );

    mkScript =
      pkgs: scriptsDir: filename:
      let
        m = match "(.+)\\.[^.]+" filename;
        name = if m != null then head m else filename;
        src = readFile (scriptsDir + "/${filename}");
        lines = final.splitString "\n" src;

        secondLine = if length lines > 1 then elemAt lines 1 else "";
        depsLine = if final.hasPrefix "# nix-deps: " secondLine then secondLine else null;
        depNames =
          if depsLine != null then
            filter (s: s != "") (final.splitString " " (final.removePrefix "# nix-deps: " depsLine))
          else
            [ ];
        depsAttrs = listToAttrs (
          map (d: {
            name = d;
            value = pkgs.${d};
          }) depNames
        );

        # Strip shebang so writeShellApplication can supply its own.
        body = final.concatStringsSep "\n" (
          if lines != [ ] && final.hasPrefix "#!" (head lines) then tail lines else lines
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

    # Build one crate from a Cargo workspace as its own package. Shares the
    # workspace source + lock (path deps resolve), but compiles/tests only `name`
    # via `cargo -p`. Installs that crate's binaries; for a lib crate it just runs
    # its tests. `version` comes from the workspace's `[workspace.package]` (member
    # crates set `version.workspace = true`, which isn't a string).
    mkRustCrate =
      pkgs: workspace: name:
      pkgs.rustPlatform.buildRustPackage {
        cargoBuildFlags = [
          "-p"
          name
        ];
        cargoLock.lockFile = workspace + "/Cargo.lock";
        cargoTestFlags = [
          "-p"
          name
        ];
        pname = name;
        src = workspace;
        version = (fromTOML (readFile (workspace + "/Cargo.toml"))).workspace.package.version;
      };

    # Package every member of the `workspace` Cargo workspace, keyed by each
    # crate's real `package.name` (mirrors mkScripts). Merge into the overlay with
    # `// lib'.lab.mkRustCrates final ./crates`; add a crate to `members` and it
    # appears as `pkgs.<name>` with no further wiring.
    mkRustCrates =
      pkgs: workspace:
      let
        members = (fromTOML (readFile (workspace + "/Cargo.toml"))).workspace.members;
        crateName = member: (fromTOML (readFile (workspace + "/${member}/Cargo.toml"))).package.name;
      in
      listToAttrs (
        map (
          member:
          let
            name = crateName member;
          in
          {
            inherit name;
            value = final.lab.mkRustCrate pkgs workspace name;
          }
        ) members
      );

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
