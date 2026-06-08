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

- `modules/default.nix` — base host options: `name` (defaults to attr name), `user` (the single home-manager user), `source` (flake URI for the `switch` alias).
- `modules/{nixos,asahi,darwin}.nix` — each adds a **platform** sub-submodule (`.nixos` / `.asahi` / `.darwin`) via `lab.mkHostPlatform`, with:
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
modules/           # flake-parts modules, auto-imported (every .nix)
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
hosts/             # one flake-parts module per host, auto-imported
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

1. Create `hosts/<name>.nix` setting `lab.hosts.<name>` with `user`, `source`, feature toggles, and per-platform `enable`/`system`/`module`/`home`.
2. Hardware/disk/user-account config goes inline in `<platform>.module`.

## Platform conditionals & key details

- Use `pkgs.stdenv.isDarwin` / `pkgs.stdenv.isLinux` inside pushed modules. Use `forLinux`/`homeLinux` vs `homeDarwin` to target platforms at the feature level.
- Overlays (`overlay.nix`) and `allowUnfree` are applied in each platform's base module — `allowUnfree = true` on **all** nixos/darwin configurations (incl. Asahi, which runs vscode/brave/obsidian). In `perSystem`, `legacyPackages` keeps `allowUnfree` off for aarch64-linux only.
- Asahi builds with `nixos-apple-silicon.inputs.nixpkgs.lib.nixosSystem` (its pinned 25.11 nixpkgs); the pin intentionally does **not** follow nixpkgs (stability — unstable kernel-panics). Because that pinned lib predates `lib.genAttrs'`, the Asahi platform uses a dedicated, era-matched `home-manager-asahi` input (`modules/asahi.nix`) instead of the main `home-manager`.
- `system.stateVersion` is **stateful** — base modules default it to `"25.05"` (the hosts' install version), independent of the current nixpkgs release. Do not bump it casually (it changed hyprland's `configType` from hyprlang→lua during the migration).
- Lynx is a remote nix builder for meerkat (`nix.sshServe` + signing keys in `keys/`).
- `eval` vs `enable`: `eval` builds a `*Configurations.<host>`; `enable` exports a `*Modules.<host>`. axolotl sets `nixos.enable = true; nixos.eval = false;` to be module-only.

## Migration verification (vs `master`)

The refactor was verified by **derivation-level** diffing (`nix-diff`) of each config against the pre-refactor `master` branch — `master` shares the same nixpkgs + home-manager revs, so the diff isolates structural changes (stronger than `nix store diff-closures`, no multi-GB build needed). To re-run: `nix-diff $(nix eval --raw "git+file://$PWD?ref=master#nixosConfigurations.lynx.config.system.build.toplevel.drvPath") $(nix eval --raw .#nixosConfigurations.lynx.config.system.build.toplevel.drvPath)`.

Result — lynx & meerkat-darwin are identical to `master` except:
- **root home-manager dropped** (intentional — single-user model).
- **home.packages reordered** — `import-tree` orders modules alphabetically vs the old explicit `home.nix` order. Same set, functionally identical, only the buildEnv hash differs (benign, unavoidable).
- one **duplicate ghostty keybind removed** (the old file listed `ctrl+shift+t` twice).

Two real regressions were caught and fixed during the diff: missing `boot.binfmt` aarch64 emulation on lynx, and wrong `stateVersion`. meerkat-**asahi** can't be diffed — `master`'s asahi doesn't evaluate (the `genAttrs'` skew), so the new config is a broken→working improvement.

Still TODO: run an actual `nixos-rebuild build`/`switch` (or `darwin-rebuild`) to deploy.

## TODO — next steps

- [ ] **Deploy**: `nixos-rebuild switch --flake .#lynx`, `darwin-rebuild switch --flake .#meerkat`, and on the Asahi box `nixos-rebuild switch --flake .#meerkat`.

Docs live in `./docs` (mdBook) — `nix run nixpkgs#mdbook -- serve docs` to preview; build output (`docs/book`) is gitignored.
