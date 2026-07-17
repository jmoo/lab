# CLAUDE.md

NixOS / nix-darwin config for multiple machines, on **flake-parts** with
`denful/import-tree` auto-importing every file under `modules/` and `hosts/`.

## Hosts

| Host | Platform | System | Notes |
|------|----------|--------|-------|
| lynx | NixOS | x86_64-linux | NVIDIA GPU, remote builder |
| meerkat | nix-darwin | aarch64-darwin | macOS on Apple Silicon |
| meerkat | NixOS (Asahi) | aarch64-linux | Apple Silicon Linux; pinned nixpkgs (unstable kernel-panics) |
| axolotl | NixOS | x86_64-linux | `nixosModules.axolotl` only, no live config |

`meerkat` is one host (`hosts/meerkat.nix`) enabling two platforms (`asahi`, `darwin`), each with its own `module` (system) and `home` (home-manager).

## Commands

- Rebuild: `sudo nixos-rebuild switch --flake .#<host>` / `darwin-rebuild switch --flake .#<host>`
- Format: `nix fmt`
- Inspect an evaluated option: `nix eval .#debug.lab.hosts.<host>.<...>` (`debug = true` exposes the module tree under `.#debug`)
- One-off tools: `nix run nixpkgs#<tool>`

## Architecture

`flake.nix` just calls `mkFlake` from `lib.nix`:

```nix
(nixpkgs.lib.extend (import ./lib.nix inputs)).mkFlake { inherit inputs; } { }
```

`lib.nix` extends `nixpkgs.lib` with a `lab` helper set and defines `mkFlake`, which runs `flake-parts.lib.mkFlake`, `import-tree`s `modules/` + `hosts/` (every `.nix` is a flake-parts module — no central import list), and exposes the extended lib to every module as `_module.args.lib'`.

### The `lab.hosts` model

Everything hangs off one option: `lab.hosts.<host>`, an `attrsOf submodule` merged from many files:

- `modules/default.nix` — base host options: `name`, `user` (the single home-manager user), `source` (flake URI for the `switch` alias), and `home` (a `deferredModule` applied to the user on **every** enabled platform).
- `modules/{nixos,asahi,darwin}.nix` — each adds a platform sub-submodule (`.nixos`/`.asahi`/`.darwin`) with `enable` (export `*Modules.<host>`), `eval` (build `*Configurations.<host>`; defaults to `enable`), `module` (system config), `home` (per-platform home-manager config), plus `system`/`specialArgs` and platform extras (e.g. asahi's `peripheralFirmwareHash`). They emit the flake outputs, splicing the host-level `home` and `<platform>.home` into `home-manager.users.${host.user}`.

### Feature modules (the core pattern)

A feature (ghostty, shell, hyprland, …) is **one file** declaring a `lab.hosts.<host>.<feature>.*` toggle and **pushing plain config down** into the relevant platforms. `lab.*` options exist *only* at the flake-parts level; pushed `module`/`home` values contain no `lab.*`.

```nix
{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeLinux; # or homeDarwin, forLinux, forAll
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.ghostty.enable = mkEnableOption "ghostty";
      config = mkIf config.ghostty.enable (homeLinux {
        programs.ghostty.enable = true;
      });
    }
  );
}
```

Push helpers (`lib.nix`): **system** → `forLinux` (nixos+asahi) / `forAll` (+darwin) set `.module`; **home** → `homeLinux` / `homeDarwin` set `.home`. For home on *all* platforms, set the host-level `home` option directly.

Notes:
- All `.home` for a platform merges into one home-manager eval, so modules compose — e.g. `ulauncher` and `hyprlock` each set their own `wayland.windowManager.hyprland.settings."$launcher"`/`"$lock"` into the `hyprland` module's settings. A feature can also default-enable companions (`hyprland` sets `waybar.enable = mkDefault true` etc.).
- Host-specific overrides go straight into the host's `home` / `<platform>.home` / `<platform>.module` — no `.common` indirection. One host, one user; no root.

### Directory layout

```
flake.nix              # calls lib.nix:mkFlake
lib.nix                # mkFlake + lab.{mkHostModule,mkHostPlatform,mkHostOptions,forLinux,forAll,homeLinux,homeDarwin,mkScript,mkScripts}
overlay.nix            # global overlay (nudelta, vscode-nix-extensions, ulauncher-uwsm, scripts/)
modules/               # flake-parts modules, auto-imported
  default.nix          #   lab.hosts base options
  {nixos,asahi,darwin}.nix  # platforms + their *{Configurations,Modules} outputs
  nixpkgs.nix          #   nixpkgs.{config,overlays} options + systems + perSystem pkgs/legacyPackages
  treefmt.nix          #   formatter (nixfmt-tree)
  {nix,locale}.nix     #   always-on shared config (nix daemon; i18n/timezone)
  <feature>.nix        #   ghostty, shell, direnv, vscode, theme, ssh, tailscale
  hyprland/            #   hyprland desktop: default.nix + greetd/ulauncher/hyprlock/
                       #     hyprpolkitagent/waybar + .conf/.json/.css assets
  iterm2/              #   iterm2 + iterm2.plist
hosts/                 # one file per host, auto-imported
keys/                  # ssh/nix pubkeys (builtins.readFile)
scripts/               # shell scripts auto-packaged by overlay.nix via lib.lab.mkScripts
pkgs/ resources/ dictionary.json
```

Non-`.nix` assets live next to their module — `import-tree` only imports `.nix`.

### Scripts (`scripts/`)

Every file in `scripts/` is automatically packaged by `overlay.nix` via `lib.lab.mkScripts`. The package name is the filename with its extension stripped (`flake-inputs.sh` → `pkgs.flake-inputs`).

Runtime dependencies are declared on **line 2** (immediately after the shebang) with a `# nix-deps:` comment:

```bash
#!/usr/bin/env bash
# nix-deps: nix jq
```

Under the hood `lib.lab.mkScript` calls `pkgs.callPackage`, so the result supports `.override { jq = jq_alt; }`. The shebang is stripped and `writeShellApplication` supplies its own (with `set -euo pipefail`).

To use a script on a host, add e.g. `pkgs.flake-inputs` to `home.packages` or `environment.systemPackages`.

### Adding things

- **Feature:** create `modules/<name>.nix` per the pattern (toggle via `mkHostModule`, gate on enable, push with the right helper), then set `lab.hosts.<host>.<name>.enable = true`. No registration — `import-tree` finds it.
- **Host:** create `hosts/<name>.nix` with `user`, `source`, cross-platform `home`, feature toggles, and per-platform `enable`/`system`/`module`/`home`. Hardware/disk/user config goes inline in `<platform>.module`.

## Code style

- **Always `nix fmt` before committing** (nixfmt-tree, 2-space).
- **Alphabetize** attribute-set keys.
- **Single child → dotted path** (`a.b.c = v;`); **2+ children → nested braces** (`a = { … };`). Decide per attrset literal; never merge across separate definitions / `mkMerge` / `mkIf`. A `mkOption { … }` call is *not* an attrset literal — never collapse it to `x.type = …`.
- Pushed config is a `{ pkgs, ... }:` function so packages resolve at build time; never reference `pkgs` in an option *default* (there is no `pkgs` at the flake-parts level).

## Working in this repo

- **Refactors must be behavior-preserving — verify it.** A pure restructure should leave the build derivations byte-identical. Capture before/after and diff:
  `nix eval --raw .#nixosConfigurations.<h>.config.system.build.toplevel.drvPath` (and `.#darwinConfigurations.<h>.system.drvPath`). Equal drvPath ⇒ provably no change. For intentional changes, `nix run nixpkgs#nix-diff -- <old> <new>` shows what moved.
- After editing, sanity-check evaluation of all configs (lynx, meerkat asahi+darwin) and that `nixosModules.axolotl` still resolves.
- **Commit in small, logical commits, and only when asked.** Branch off `master` first if needed.

## Key details

- **nixpkgs config + overlays** are top-level options (`nixpkgs.config` = `attrsOf anything`, `nixpkgs.overlays` = `listOf raw`) in `modules/nixpkgs.nix`, applied to every host platform **and** to `perSystem` pkgs/`legacyPackages` — one source of truth. (`overlays` must be `types.raw`: opaque, so the module system never forces the overlay functions, which would infinite-recurse via `option → pkgs → flake outputs → config → option`.) `allowUnfree = true` everywhere (incl. Asahi).
- **Asahi** uses the pinned `nixos-apple-silicon` nixpkgs (25.11; intentionally not following nixpkgs). Its lib predates `lib.genAttrs'`, so the platform uses a dedicated era-matched `home-manager-asahi` input.
- **`system.stateVersion`** is stateful — pinned to `"25.05"` (install version), not the nixpkgs release. Don't bump casually (flips hyprland `configType` hyprlang→lua at 26.05).
- **lynx** is a remote builder for meerkat (`nix.sshServe` + signing keys in `keys/`).
