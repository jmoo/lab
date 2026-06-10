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

Meerkat is one host (`hosts/meerkat.nix`) that enables two platforms — `asahi` and `darwin` — each with its own `module` (system) and `home` (home-manager) config.

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
- imports **every `.nix` file** under `modules/` and `hosts/` via `import-tree` (so each file is a flake-parts module — there is no central import list),
- exposes the extended lib to every module as `_module.args.lib'`.

### The `lab.hosts` model

Everything hangs off one flake-parts option: `lab.hosts.<host>`, an `attrsOf submodule`. The submodule is assembled by merging declarations from many files:

- `modules/default.nix` — base host options: `name` (defaults to attr name), `user` (the single home-manager user), `source` (flake URI for the `switch` alias), and `home` (a `deferredModule` of home-manager config applied to the user on **every** enabled platform — use it for cross-platform home config instead of repeating it per platform).
- `modules/{nixos,asahi,darwin}.nix` — each adds a **platform** sub-submodule (`.nixos` / `.asahi` / `.darwin`) via `lab.mkHostPlatform`, with:
  - `enable` — export this platform as a `*Modules.<host>` output,
  - `eval` — build a full `*Configurations.<host>` (defaults to `enable`),
  - `module` — a `deferredModule` holding the platform's **system** config,
  - `home` — a `deferredModule` holding that platform's **home-manager** config,
  - plus `system`, `specialArgs`, and platform-specific options (e.g. asahi's `peripheralFirmwareHash`).
  These files also emit the flake outputs by mapping over `config.lab.hosts` (filtered by `eval`/`enable`) into `nixosSystem` / `darwinSystem`, splicing both the host-level `home` and the `<platform>.home` into `home-manager.users.${host.user}`.

### Feature modules (the important pattern)

A feature (ghostty, shell, hyprland, …) is **one file** that declares its toggle as a `lab.hosts.<host>.<feature>.*` option and **pushes config down** into the relevant platforms. `lab.*` options exist *only* at this flake-parts level; the pushed-down `module`/`home` values are plain NixOS / home-manager config with no `lab.*` inside them.

```nix
{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeLinux;   # or homeDarwin, forLinux/forAll
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
- **Home** config → platform `.home`: `homeLinux`, `homeDarwin`. For **all** platforms, set the host-level `home` option directly (`config.home = …`) instead of a helper.

Because every feature's pushed `.home` for a platform merges into one home-manager evaluation, co-resident home modules can share option namespaces (see the desktop bundle's `apps` set + `wrapHyprCommand` arg). Pushed config is usually written as a `{ pkgs, ... }:` function so packages resolve at build time (there is no `pkgs` at the flake-parts level — option *defaults* must not reference `pkgs`).

Host-specific overrides are written directly into a host's `home` (cross-platform), `<platform>.home` (one platform), or `<platform>.module` (system) — e.g. meerkat's shared home packages live in `home`, its asahi-only swaylock/HiDPI in `asahi.home`, and lynx's ghostty theme + `hyprland.nvidia` in `nixos.home`. There is no multi-user/root support — one host, one user.

### Directory layout

```
flake.nix              # calls lib.nix:mkFlake
lib.nix                # mkFlake + lab.{mkHostModule,mkHostPlatform,mkHostOptions,forLinux,forAll,homeLinux,homeDarwin}
overlay.nix            # global overlay (nudelta, vscode-nix-extensions, ulauncher-uwsm)
modules/               # flake-parts modules, auto-imported (every .nix)
  default.nix          # lab.hosts base options + home-manager flakeModule
  nixos.nix            # nixos platform + nixos{Configurations,Modules}
  asahi.nix            # asahi platform (pinned nixpkgs) + nixos{Configurations,Modules}
  darwin.nix           # darwin platform + darwin{Configurations,Modules}
  nixpkgs.nix          # nixpkgs.{config,overlays} options + systems + perSystem pkgs/legacyPackages
  treefmt.nix          # perSystem formatter (nixfmt-tree)
  locale.nix           # shared (always-on): i18n + time.timeZone (Linux)
  nix.nix              # shared (always-on): nix daemon settings
  <feature>.nix        # ghostty, shell, direnv, vscode, iterm2,
                       #   greetd, ssh, tailscale
  hyprland/            # desktop bundle (hypridle/hyprlock/hyprpaper/hyprpolkitagent/
                       #   apps/theme/ulauncher/waybar/nm-applet) + config/ & waybar/ assets
  iterm2/              # iterm2 feature + iterm2.plist
hosts/                 # one flake-parts module per host, auto-imported
keys/                  # ssh/nix pubkeys (referenced via builtins.readFile)
pkgs/                  # custom packages + vscode-nix-extensions home-manager module
resources/             # vscode themes/icons, mascots
dictionary.json        # cSpell words for vscode
```

Non-`.nix` assets (`*.conf`, `*.json`, `*.css`, `*.plist`, pubkeys) live next to their module (or in `keys/`) — `import-tree` only imports `.nix`, so they are never mistaken for flake-parts modules.

### Adding a feature module

1. Create `modules/<name>.nix` following the pattern above: declare `lab.hosts.<host>.<name>.*` options via `mkHostModule`, gate config on the enable, and push it with the right helper (`homeLinux`/`forAll`/etc.).
2. Enable it per host: `lab.hosts.<host>.<name>.enable = true;`.
   No central registration — `import-tree` picks the file up automatically.

### Adding a host

1. Create `hosts/<name>.nix` setting `lab.hosts.<name>` with `user`, `source`, a cross-platform `home`, feature toggles, and per-platform `enable`/`system`/`module`/`home`.
2. Hardware/disk/user-account config goes inline in `<platform>.module`.

## Platform conditionals & key details

- Use `pkgs.stdenv.isDarwin` / `pkgs.stdenv.isLinux` inside pushed modules. Use `forLinux`/`homeLinux` vs `homeDarwin` to target platforms at the feature level.
- **nixpkgs config + overlays** are top-level flake-parts options — `nixpkgs.config` (`attrsOf anything`) and `nixpkgs.overlays` (`listOf raw`) — declared and defaulted in `modules/nixpkgs.nix`. `nixpkgs.nix` applies them to every host platform (via `forAll`) **and** to the `perSystem` `pkgs`/`legacyPackages`, so it's one source of truth. (`overlays` must be `types.raw`, not `types.anything`: `raw` is opaque, so the module system never forces the overlay functions, which would otherwise infinite-recurse via `option → pkgs → flake outputs → config → option`.) `allowUnfree = true` everywhere (incl. Asahi, which runs vscode/brave/obsidian, and `legacyPackages`).
- nix daemon settings and locale/timezone come from always-on shared modules (`nix.nix`, `locale.nix`) that push to every platform with no enable toggle.
- Asahi builds with `nixos-apple-silicon.inputs.nixpkgs.lib.nixosSystem` (its pinned 25.11 nixpkgs); the pin intentionally does **not** follow nixpkgs (stability — unstable kernel-panics). Because that pinned lib predates `lib.genAttrs'`, the Asahi platform uses a dedicated, era-matched `home-manager-asahi` input (`modules/asahi.nix`) instead of the main `home-manager`.
- `system.stateVersion` is **stateful** — base modules default it to `"25.05"` (the hosts' install version), independent of the current nixpkgs release. Do not bump it casually (e.g. it flips hyprland's `configType` from hyprlang to lua at 26.05).
- Lynx is a remote nix builder for meerkat (`nix.sshServe` + signing keys in `keys/`).
- `eval` vs `enable`: `eval` builds a `*Configurations.<host>`; `enable` exports a `*Modules.<host>`. axolotl sets `nixos.enable = true; nixos.eval = false;` to be module-only.
