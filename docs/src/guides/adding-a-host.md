# Adding a host

1. Create `hosts/<name>.nix`:

   ```nix
   { ... }:
   {
     lab.hosts.<name> = {
       user = "jmoore";
       source = "/home/jmoore/Repos/jmoo/lab";

       # feature toggles
       ghostty.enable = true;
       shell.enable = true;
       # ...

       nixos = {
         enable = true;
         system = "x86_64-linux";

         home = { /* per-user home overrides */ };

         module =
           { pkgs, lib, modulesPath, ... }:
           {
             # hardware, fileSystems, users.users.<user>, networking, services …
           };
       };
     };
   }
   ```

2. Hardware/disk/user-account config goes inline in `<platform>.module`
   (there is no separate `hardware.nix`; inline it or import a path outside the
   `import-tree` roots).

3. Platforms:
   - `nixos` — standard NixOS host.
   - `asahi` — Apple Silicon Linux; also set `peripheralFirmwareHash`.
   - `darwin` — macOS; a host may enable several platforms at once (see
     [meerkat](../hosts/meerkat.md)).

4. `eval` vs `enable`:
   - `enable = true` exports `*Modules.<name>`.
   - `eval` (defaults to `enable`) builds `*Configurations.<name>`. Set
     `eval = false` to export a module without an active configuration (see
     [axolotl](../hosts/axolotl.md)).

5. Verify before deploying:

   ```sh
   nix eval .#nixosConfigurations.<name>.config.system.build.toplevel.drvPath
   nix flake check
   ```
