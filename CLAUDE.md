# CLAUDE.md

## What this is

NixOS/nix-darwin configuration repo managing multiple machines across platforms. Uses flakes.

## Hosts

| Host | Platform | System | Notes |
|------|----------|--------|-------|
| lynx | NixOS | x86_64-linux | NVIDIA GPU, remote builder |
| meerkat | nix-darwin | aarch64-darwin | macOS on Apple Silicon |
| meerkat | NixOS (Asahi) | aarch64-linux | Linux on Apple Silicon, uses pinned nixos-apple-silicon nixpkgs (not unstable — kernel panic) |
| axolotl | NixOS | x86_64-linux | Exported as nixosModule only, no active nixosConfiguration |

Meerkat has two configs sharing `hosts/meerkat/common.nix` — `darwin.nix` for macOS, `asahi.nix` for Linux.

## Rebuild commands

- NixOS: `sudo nixos-rebuild switch --flake .`
- Darwin: `darwin-rebuild switch --flake .`
- Format: `nix fmt`

## The `lab` options system

The core abstraction is in `modules/common.nix`. It auto-generates `lab.<module>` options for each module in the `passthru` list:

```
passthru = [ "apps" "direnv" "ghostty" "hyprland" "hyprpaper" "hypridle"
             "hyprlock" "iterm2" "karabiner" "theme" "shell" "vscode" "waybar" ];
```

For each passthru module, these options are created:
- `lab.<mod>.enable` — toggle the module
- `lab.<mod>.users` — which users get it (defaults to `lab.users`)
- `lab.<mod>.root` — also apply to root
- `lab.<mod>.common` — extra config merged into every targeted user

Top-level `lab` options:
- `lab.users` — list of non-root users
- `lab.root` — apply base config to root
- `lab.common` — base home-manager config for all users
- `lab.name` — defaults to hostname
- `lab.source` — flake URI for rebuild aliases

### How it flows

1. Host sets `lab.users`, `lab.<mod>.enable`, etc.
2. `common.nix` builds `home-manager.users` via `foldl'` over `lab.users ++ optional root`
3. Each user gets `lab.common` applied, which imports `home.nix` (all passthru module implementations)
4. Per-module enable flags are resolved per-user: enabled only if `lab.<mod>.enable && (user in lab.<mod>.users || root)`
5. `lab.<mod>.common` is merged into each targeted user's module config

### Adding a new lab module

1. Create `modules/<name>.nix` (or directory) with `lab.<name>.enable` option gating its config
2. Add `"<name>"` to the `passthru` list in `modules/common.nix`
3. Add the import to `modules/home.nix`
4. Enable it in host configs: `lab.<name>.enable = true`

For system-level modules (not home-manager), don't use passthru — define options directly and import from `modules/nixos.nix`, `modules/darwin.nix`, or `modules/asahi.nix` as appropriate. See `greetd.nix`, `ssh.nix`, `k3s.nix` for examples. Every module needs to have an enable option.

## Module hierarchy

```
flake.nix
  ├─ modules/nixos.nix      # NixOS platform (imports common.nix, home.nix, greetd, ssh, etc.)
  ├─ modules/darwin.nix      # macOS platform (imports common.nix, home.nix)
  ├─ modules/asahi.nix       # Asahi Linux (imports nixos.nix + apple-silicon-support)
  └─ modules/common.nix      # Lab option generation + home-manager user wiring
       ├─ modules/lab.nix    # Base lab.name, lab.source options
       ├─ modules/nix.nix    # Nix daemon settings
       └─ modules/home.nix   # Imports all passthru module implementations
```

## Platform conditionals

Use `pkgs.stdenv.isDarwin` / `pkgs.stdenv.isLinux` in module code. See `shell.nix` for examples.

Host-level overrides use `mkForce` when a shared `common.nix` sets a default that needs platform-specific changes.

## Key details

- `specialArgs = { inherit inputs; }` passes flake inputs to all modules
- Overlays are in `overlay.nix`, applied globally to all configurations
- `allowUnfree` is enabled on darwin and x86_64-linux but NOT aarch64-linux (Asahi)
- Lynx serves as a remote nix builder for meerkat (nix-ssh + signing keys)
- nixos-apple-silicon does NOT follow nixpkgs (intentional — stability)
