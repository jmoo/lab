# lab

Playground for personal projects and NixOS / nix-darwin config for my machines.

## Hosts

| | Host | Description |
|---|------|-------------|
| 🐆 | lynx | NixOS workstation with NVIDIA GPU |
| 🦫 | meerkat | Apple Silicon laptop — nix-darwin and Asahi Linux dual boot |
| 🦎 | axolotl | Base NixOS module for some downstream devices |
| 🦡 | badger | home-manager on Termux/Android (Boox Note Max) |

## Projects

| | Name | Description |
|---|------|-------------|
| 🎴 | [anki-tool](crates/anki-tool/README.md) | CLI for querying AnkiConnect, designed for AI agent consumption |
| 🎹 | [nord-format](crates/nord-format/README.md) | Clavia / Nord file parser/writer implementation in rust |
| 🎛️ | [nord-cli](crates/nord-cli/README.md) | Command-line tool for interacting with Clavia / Nord keyboards and files |
| 🔌 | [nord-usb](crates/nord-usb/README.md) | Clavia / Nord USB protocol implementation in rust |
| 🌐 | [nord-web-demo](crates/nord-web-demo/README.md) | Browser page driving nord-usb's WebUSB backend on real hardware |
| 🧩 | [vscode-nix-extensions](pkgs/vscode-nix-extensions/README.md) | Generate VS Code extensions from Nix expressions |

## The Nord specimen corpus

The `nord-*` crates are tested against a specimen corpus in a **separate private
repo**, pinned by revision in `overlay.nix` and fetched over SSH only when a
corpus target is built. `NORD_CORPUS_DIR` names the corpus root — the directory
holding `ne5/`, `ne6/`, … and `library/`.

```sh
nix build .#checks.<system>.nord-format-corpus   # needs read access to the corpus repo
nix build .#checks.<system>.nord-usb-corpus
nix build .#checks.<system>.corpus-rev-agrees    # needs nothing
```

`pkgs.nord-corpus` is the corpus's own `nix/corpus.nix` assembly, not the raw
tree: the git tier filtered against the corpus's R2 manifest, with the corpus
repo's standing assertion that nothing oversized escaped it. Only that output is
exposed, so no blob address from the manifest reaches this public repo.

### Building the corpus with its R2-tier blobs

`packages.nord-corpus-full` is the same assembly with the manifest's private
blobs — the multi-hundred-megabyte bundle archives and their untrimmed captures —
spliced in. It is deliberately **not** a check: those blobs live in a private
bucket, and `nix flake check` has to run without credentials for it.

Each blob is a fixed-output derivation whose output *is* the blob, so the simplest
way to build it needs no privilege change — fetch the blobs yourself and hand them
to the store at exactly the path the derivation expects:

```sh
cd ~/Repos/jmoo/nord-corpus && nix develop
corpus nix-add <artifact-id>          # once per blob; `corpus doctor` checks R2 setup

nix build .#nord-corpus-full
```

Once a blob's store path exists Nix considers that derivation satisfied and never
runs the fetch. The alternative is to give the Nix daemon the `R2_*` variables (in
a multi-user install it does not inherit yours), which the derivation's own error
message spells out.

> ⚠️ Never push `nord-corpus-full` or its blobs to a shared binary cache. The
> outputs are vendor firmware and piano libraries under their own hashes, and
> content-addressing is not access control.

## Disclaimer

Not affiliated with, authorized, or endorsed by Clavia DMI AB. (https://www.nordkeyboards.com) 
"Nord", "Clavia", and "Electro" are trademarks of Clavia DMI AB, used here only to identify the
hardware these formats come from. All Clavia / Nord artifacts included in this repo
are synthetic test artifacts produced by the author of this repo.