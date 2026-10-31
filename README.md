# lab

Playground for personal projects and NixOS / nix-darwin config for my machines.

## Website

[jmoo.io](https://jmoo.io) lives in [sites/jmoo.io/index.html](sites/jmoo.io/index.html).
The page is one self-contained HTML file, with no build step or external dependencies.
Preview it by opening the file in a browser, or run
`python3 -m http.server 8000 --directory sites/jmoo.io`.

The Pages workflow deploys only `sites/jmoo.io/` when that directory or the workflow
changes on `master`; it can also be run manually. For initial setup, select
**GitHub Actions** under **Settings → Pages → Build and deployment**, set the custom
domain to `jmoo.io`, and configure its DNS using
[GitHub’s apex domain instructions](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/managing-a-custom-domain-for-your-github-pages-site#configuring-an-apex-domain).
Enable **Enforce HTTPS** once the certificate is ready. Actions deployments use
the custom domain in Pages settings and do not require a `CNAME` file.

## Hosts

| | Host | Description |
|---|------|-------------|
| 🐆 | lynx | NixOS workstation with NVIDIA GPU |
| 🦫 | meerkat | Apple Silicon laptop — nix-darwin and Asahi Linux dual boot |
| 🦎 | axolotl | Base NixOS module for some downstream devices |
| 🦡 | badger | home-manager on Termux/Android (Boox Note Max) |

## Projects

#### In this repo

| | Name | Description |
|---|------|-------------|
| 🎴 | [anki-tool](crates/anki-tool/README.md) | CLI for querying AnkiConnect, designed for AI agent consumption |
| 🧩 | [vscode-nix-extensions](pkgs/vscode-nix-extensions/README.md) | Generate VS Code extensions from Nix expressions |

#### Graduated 

| | Name | Description |
|---|------|-------------|
| 🎚️ | [drawbar](https://github.com/jmoo/drawbar) | Read, edit and move the sounds on your Nord keyboard. In your browser, from your terminal, or from your own code. |
