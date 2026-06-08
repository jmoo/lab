# CLAUDE.md

## What this is

NixOS/nix-darwin configuration repo managing multiple machines across platforms.
Built on **flake-parts** with `denful/import-tree` auto-importing every module and host.

## Hosts

| Host | Platform | System | Notes |
|------|----------|--------|-------|
| lynx | NixOS | x86_64-linux | NVIDIA GPU, remote builder |
| meerkat | nix-darwin | aarch64-darwin | macOS on Apple Silicon |
| meerkat | NixOS (Asahi) | aarch64-linux | Linux on Apple Silicon, uses pinned nixos-apple-silicon nixpkgs (not unstable — kernel panic) |
| axolotl | NixOS | x86_64-linux | Exported as `nixosModules.axolotl` only, no active nixosConfiguration |

Meerkat is one host (`new-hosts/meerkat.nix`) that enables two platforms — `asahi` and `darwin` — each with its own `module` (system) and `home` (home-manager) config.

## Rebuild commands

- NixOS: `sudo nixos-rebuild switch --flake .#<host>`
- Darwin: `darwin-rebuild switch --flake .#<host>`
- Format: `nix fmt`
- Inspect merged options: `nix eval .#debug.lab.hosts.<host>.<...>` (flake-parts `debug = true` exposes the evaluated module tree under `.#debug`)

## Architecture

`flake.nix` is tiny — it calls `mkFlake` from `lib.nix`:

```nix
(nixpkgs.lib.extend (import ./lib.nix inputs)).mkFlake { inherit inputs; } { }
```

`lib.nix` extends `nixpkgs.lib` with a `lab` helper set and defines `mkFlake`, which:
- runs `flake-parts.lib.mkFlake`,
- imports **every `.nix` file** under `new-modules/` and `new-hosts/` via `import-tree` (so each file is a flake-parts module — there is no central import list),
- exposes the extended lib to every module as `_module.args.lib'`.

### The `lab.hosts` model

Everything hangs off one flake-parts option: `lab.hosts.<host>`, an `attrsOf submodule`. The submodule is assembled by merging declarations from many files:

- `new-modules/default.nix` — base host options: `name` (defaults to attr name), `user` (the single home-manager user), `source` (flake URI for the `switch` alias).
- `new-modules/{nixos,asahi,darwin}.nix` — each adds a **platform** sub-submodule (`.nixos` / `.asahi` / `.darwin`) via `lab.mkHostPlatform`, with:
  - `enable` — export this platform as a `*Modules.<host>` output,
  - `eval` — build a full `*Configurations.<host>` (defaults to `enable`),
  - `module` — a `deferredModule` holding the platform's **system** config,
  - `home` — a `deferredModule` holding that platform's **home-manager** config,
  - plus `system`, `specialArgs`, and platform-specific options (e.g. asahi's `peripheralFirmwareHash`).
  These files also emit the flake outputs by mapping over `config.lab.hosts` (filtered by `eval`/`enable`) into `nixosSystem` / `darwinSystem`, splicing `<platform>.home` into `home-manager.users.${host.user}`.

### Feature modules (the important pattern)

A feature (ghostty, shell, hyprland, …) is **one file** that declares its toggle as a `lab.hosts.<host>.<feature>.*` option and **pushes config down** into the relevant platforms. `lab.*` options exist *only* at this flake-parts level; the pushed-down `module`/`home` values are plain NixOS / home-manager config with no `lab.*` inside them.

```nix
{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeLinux;   # or homeAll/homeDarwin, forLinux/forAll
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.ghostty.enable = mkEnableOption "ghostty";

      config = mkIf config.ghostty.enable (homeLinux {
        programs.ghostty = { enable = true; /* ... */ };
      });
    }
  );
}
```

Push helpers (in `lib.nix`):
- **System** config → platform `.module`: `forLinux` (nixos + asahi), `forAll` (+ darwin).
- **Home** config → platform `.home`: `homeLinux`, `homeAll`, `homeDarwin`.

Because every feature's pushed `.home` for a platform merges into one home-manager evaluation, co-resident home modules can share option namespaces (see the desktop bundle's `apps` set + `wrapHyprCommand` arg). Pushed config is usually written as a `{ pkgs, ... }:` function so packages resolve at build time (there is no `pkgs` at the flake-parts level — option *defaults* must not reference `pkgs`).

Host-specific overrides are written the old-fashioned way directly into a host's `<platform>.module` / `<platform>.home` (e.g. lynx's ghostty theme + `hyprland.nvidia`, meerkat-asahi's swaylock + HiDPI). There is no `.common` option and no multi-user/root support — one host, one user.

### Directory layout

```
flake.nix              # calls lib.nix:mkFlake
lib.nix                # mkFlake + lab.{mkHostModule,mkHostPlatform,mkHostOptions,forLinux,forAll,homeLinux,homeAll,homeDarwin}
overlay.nix            # global overlay (nudelta, vscode-nix-extensions, ulauncher-uwsm)
new-modules/           # flake-parts modules, auto-imported (every .nix)
  default.nix          # lab.hosts base options + home-manager flakeModule
  nixos.nix            # nixos platform + nixos{Configurations,Modules}
  asahi.nix            # asahi platform (pinned nixpkgs) + nixos{Configurations,Modules}
  darwin.nix           # darwin platform + darwin{Configurations,Modules}
  perSystem.nix        # systems, formatter (nixfmt-tree), legacyPackages, overlays.default
  <feature>.nix        # ghostty, shell, direnv, vscode, iterm2, karabiner,
                       #   greetd, ssh, k3s, tailscale
  hyprland/            # desktop bundle (hypridle/hyprlock/hyprpaper/hyprpolkitagent/
                       #   apps/theme/ulauncher/waybar/nm-applet) + config/ & waybar/ assets
  iterm2/              # iterm2 feature + iterm2.plist
new-hosts/             # one flake-parts module per host, auto-imported
keys/                  # ssh/nix pubkeys (referenced via builtins.readFile)
pkgs/                  # custom packages + vscode-nix-extensions home-manager module
resources/             # vscode themes/icons, mascots
dictionary.json        # cSpell words for vscode
```

Non-`.nix` assets (`*.conf`, `*.json`, `*.css`, `*.plist`, pubkeys) live next to their module (or in `keys/`) — `import-tree` only imports `.nix`, so they are never mistaken for flake-parts modules.

### Adding a feature module

1. Create `new-modules/<name>.nix` following the pattern above: declare `lab.hosts.<host>.<name>.*` options via `mkHostModule`, gate config on the enable, and push it with the right helper (`homeLinux`/`forAll`/etc.).
2. Enable it per host: `lab.hosts.<host>.<name>.enable = true;`.
   No central registration — `import-tree` picks the file up automatically.

### Adding a host

1. Create `new-hosts/<name>.nix` setting `lab.hosts.<name>` with `user`, `source`, feature toggles, and per-platform `enable`/`system`/`module`/`home`.
2. Hardware/disk/user-account config goes inline in `<platform>.module`.

## Platform conditionals & key details

- Use `pkgs.stdenv.isDarwin` / `pkgs.stdenv.isLinux` inside pushed modules. Use `forLinux`/`homeLinux` vs `homeDarwin` to target platforms at the feature level.
- Overlays (`overlay.nix`) and `allowUnfree` are applied in each platform's base module and in `perSystem`. `allowUnfree` is on for darwin + x86_64-linux but **NOT** aarch64-linux (Asahi).
- Asahi builds with `nixos-apple-silicon.inputs.nixpkgs.lib.nixosSystem` (its pinned nixpkgs); the pin intentionally does **not** follow nixpkgs (stability — unstable kernel-panics).
- Lynx is a remote nix builder for meerkat (`nix.sshServe` + signing keys in `keys/`).
- `eval` vs `enable`: `eval` builds a `*Configurations.<host>`; `enable` exports a `*Modules.<host>`. axolotl sets `nixos.enable = true; nixos.eval = false;` to be module-only.

## TODO — next steps

- [ ] **Fix meerkat Asahi evaluation.** `home-manager`'s `qt.nix` uses `lib.genAttrs'`, which the pinned Asahi nixpkgs lib lacks (main nixpkgs has it). Pre-existing input skew — not module code. Pin `home-manager` to a release matching the Asahi nixpkgs, or bump the Asahi pin. (lynx + meerkat-darwin both evaluate fine.)
- [ ] **Full build verification + closure diff.** Only evaluation (`*.drvPath`) has been checked. Build the configs (`nixos-rebuild build --flake .#lynx`, `darwin-rebuild build --flake .#meerkat`) and **assert the refactor did not change the output closures** vs the pre-refactor config:
  - Build the old toplevel from the pre-migration revision, e.g. `nix build "git+file://$PWD?ref=master#nixosConfigurations.lynx.config.system.build.toplevel" -o result-old` (or a pre-flake-parts commit), and the new one `nix build .#nixosConfigurations.lynx.config.system.build.toplevel -o result-new`.
  - Compare: `nix store diff-closures ./result-old ./result-new` (and/or `nvd diff result-old result-new`). A pure refactor should show **no** package/version changes.
  - Caveat: pin **identical `flake.lock` inputs** on both sides first, otherwise nixpkgs/home-manager bumps will show up as diffs and mask the structural comparison. Then switch/deploy.
- [ ] **Rename `new-modules/` → `modules/` and `new-hosts/` → `hosts/`** once the pattern is settled, and update the two `import-tree` paths in `lib.nix`.
- [ ] **Commit the migration** (currently staged, not committed).
- [ ] **Remove karabiner** (`new-modules/karabiner.nix`) — ported for completeness but no host enables it.
- [ ] Consider standalone **`homeConfigurations`** output (the old single-platform `home` module was dropped in favor of per-platform `<platform>.home`).
- [ ] **Create a `./docs` directory with an mdBook** containing the old per-host READMEs (mascot art from `resources/mascots/`) plus additional documentation.
