# jmoo/lab

NixOS / nix-darwin configuration managing multiple machines across platforms,
built on [flake-parts](https://flake.parts) with
[`denful/import-tree`](https://github.com/denful/import-tree) auto-importing
every module and host.

| Host | Platform | System | Notes |
|------|----------|--------|-------|
| [lynx](./hosts/lynx.md) | NixOS | x86_64-linux | NVIDIA GPU, remote builder |
| [meerkat](./hosts/meerkat.md) | nix-darwin | aarch64-darwin | macOS on Apple Silicon |
| [meerkat](./hosts/meerkat.md) | NixOS (Asahi) | aarch64-linux | Linux on Apple Silicon (pinned nixpkgs) |
| [axolotl](./hosts/axolotl.md) | NixOS | x86_64-linux | Exported as a module only |

## Rebuild commands

```sh
# NixOS
sudo nixos-rebuild switch --flake .#<host>

# Darwin
darwin-rebuild switch --flake .#<host>

# Format
nix fmt
```

## Repository map

```
flake.nix     # calls lib.nix:mkFlake
lib.nix       # mkFlake + the lab.* helper set
overlay.nix   # global package overlay
modules/      # flake-parts modules (auto-imported)
hosts/        # one flake-parts module per host (auto-imported)
keys/         # ssh/nix pubkeys
pkgs/         # custom packages
resources/    # vscode themes/icons, mascots
docs/         # this book
```

> The canonical, always-current reference for contributors is
> [`CLAUDE.md`](https://github.com/jmoo/lab/blob/master/CLAUDE.md) at the repo
> root. This book mirrors the high points and adds per-host notes.
