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

The `nord-*` crates are tested against a specimen corpus in a separate private repo (`jmoo/nord-corpus`), pinned by revision in `overlay.nix`. Each crate's README says how to run its tests.

## Disclaimer

Not affiliated with, authorized, or endorsed by Clavia DMI AB. (https://www.nordkeyboards.com) 
"Nord", "Clavia", and "Electro" are trademarks of Clavia DMI AB, used here only to identify the
hardware these formats come from. All Clavia / Nord artifacts included in this repo
are synthetic test artifacts produced by the author of this repo.