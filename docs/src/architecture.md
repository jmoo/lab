# Architecture

`flake.nix` is tiny — it calls `mkFlake` from `lib.nix`:

```nix
(nixpkgs.lib.extend (import ./lib.nix inputs)).mkFlake { inherit inputs; } { }
```

`lib.nix` extends `nixpkgs.lib` with a `lab` helper set and defines `mkFlake`,
which:

- runs `flake-parts.lib.mkFlake`,
- imports **every `.nix` file** under `modules/` and `hosts/` via `import-tree`
  (each file is a flake-parts module — there is no central import list),
- exposes the extended lib to every module as `_module.args.lib'`.

## The `lab.hosts` model

Everything hangs off one flake-parts option: `lab.hosts.<host>`, an
`attrsOf submodule`. The submodule is assembled by merging declarations from
many files:

- `modules/default.nix` — base host options: `name`, `user` (the single
  home-manager user), `source` (flake URI for the `switch` alias).
- `modules/{nixos,asahi,darwin}.nix` — each adds a **platform** sub-submodule
  (`.nixos` / `.asahi` / `.darwin`) with:
  - `enable` — export this platform as a `*Modules.<host>` output,
  - `eval` — build a full `*Configurations.<host>` (defaults to `enable`),
  - `module` — a `deferredModule` holding the platform's **system** config,
  - `home` — a `deferredModule` holding that platform's **home-manager** config,
  - plus `system`, `specialArgs`, and platform-specific options.

  These files also emit the flake outputs by mapping over `config.lab.hosts`
  (filtered by `eval`/`enable`) into `nixosSystem` / `darwinSystem`, splicing
  `<platform>.home` into `home-manager.users.${host.user}`.

## Feature modules

A feature (ghostty, shell, hyprland, …) is **one file** that declares its
toggle as a `lab.hosts.<host>.<feature>.*` option and **pushes config down**
into the relevant platforms. `lab.*` options exist *only* at this flake-parts
level; the pushed-down `module`/`home` values are plain NixOS / home-manager
config with no `lab.*` inside them.

```nix
{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeLinux;
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

Push helpers (from `lib.nix`):

| Helper | Target |
|--------|--------|
| `forLinux` / `forAll` | **system** config → `nixos`/`asahi`(+`darwin`) `.module` |
| `homeLinux` / `homeAll` / `homeDarwin` | **home** config → platform `.home` |

Because every feature's pushed `.home` for a platform merges into one
home-manager evaluation, co-resident home modules can share option namespaces
(see the desktop bundle's `apps` set + `wrapHyprCommand` arg). Pushed config is
usually a `{ pkgs, ... }:` function so packages resolve at build time — there is
no `pkgs` at the flake-parts level, so option *defaults* must not reference it.

## Key details

- **Overlays** (`overlay.nix`) and `allowUnfree` are applied in every platform's
  base module. `legacyPackages` keeps `allowUnfree` off for aarch64-linux only.
- **Asahi** builds with the pinned `nixos-apple-silicon` nixpkgs (25.11; the pin
  does not follow nixpkgs — unstable kernel-panics). Its lib predates
  `lib.genAttrs'`, so the Asahi platform uses a dedicated, era-matched
  `home-manager-asahi` input.
- **`stateVersion` is stateful** — pinned to the hosts' install version
  (`25.05`), independent of the current nixpkgs release.
- **`eval` vs `enable`**: `eval` builds a `*Configurations.<host>`; `enable`
  exports a `*Modules.<host>`. axolotl sets `nixos.enable = true; eval = false;`.
