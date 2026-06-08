# Adding a feature module

A feature is one file under `modules/` that declares a host-level toggle and
pushes config into the relevant platforms. `import-tree` picks it up
automatically — there is no central registration.

1. Create `modules/<name>.nix`:

   ```nix
   { lib', ... }:
   let
     inherit (lib'.lab) mkHostModule homeAll; # or homeLinux/homeDarwin, forLinux/forAll
     inherit (lib') mkEnableOption mkIf;
   in
   {
     options.lab.hosts = mkHostModule (
       { config, ... }:
       {
         options.<name>.enable = mkEnableOption "<name>";

         config = mkIf config.<name>.enable (homeAll (
           { pkgs, ... }:
           {
             # plain home-manager config — no lab.* here
           }
         ));
       }
     );
   }
   ```

2. Pick the push helper for the target:
   - **home** config → `homeAll` / `homeLinux` / `homeDarwin`
   - **system** config → `forAll` / `forLinux` (writes the platform `.module`)
   - A feature can do both (e.g. set `nixos.module` *and* `nixos.home`).

3. Enable it per host: `lab.hosts.<host>.<name>.enable = true;`.

## Conventions

- `lab.*` options live **only** at the flake-parts level. The pushed `module` /
  `home` values are pure NixOS / home-manager config.
- Write pushed config as `{ pkgs, ... }:` so packages resolve at build time;
  never reference `pkgs` in an option *default* (there is no `pkgs` at the
  flake-parts level — make package options `nullOr package` and resolve a
  fallback inside the module).
- Assets (`.conf`, `.json`, `.css`, …) live next to the module; `import-tree`
  only imports `.nix`, so they are never treated as flake-parts modules.
- Host-specific tweaks are written the old-fashioned way directly into the
  host's `<platform>.module` / `<platform>.home`.
