# meerkat

![meerkat](../mascots/meerkat.jpg)

A single host (`hosts/meerkat.nix`) that enables **two platforms** — `darwin`
(macOS) and `asahi` (Linux on Apple Silicon) — each with its own `module`
(system) and `home` (home-manager) config. Shared bits (vscode, direnv, shell,
home packages, the lynx build-offload `nix` settings) are set once at the host
level.

## meerkat — darwin

```
                    c.'          jmoore@meerkat
                 ,xNMM.          --------------
               .OMMMMo           OS: macOS arm64
               lMM"              Host: Mac14,5
     .;loddo:.  .olloddol;.      CPU: Apple M2 Max
   cKMMMMMMMMMMNWMMMMMMMMMM0:    GPU: Apple M2 Max
 .KMMMMMMMMMMMMMMMMMMMMMMMWd.    Memory: 64 GB
 XMMMMMMMMMMMMMMMMMMMMMMMX.      WM: Quartz Compositor
;MMMMMMMMMMMMMMMMMMMMMMMM:       Terminal: iTerm2
:MMMMMMMMMMMMMMMMMMMMMMMM:       Shell: zsh
.MMMMMMMMMMMMMMMMMMMMMMMMX.
 kMMMMMMMMMMMMMMMMMMMMMMMMWd.
 'XMMMMMMMMMMMMMMMMMMMMMMMMMMk
  'XMMMMMMMMMMMMMMMMMMMMMMMMK.
    kMMMMMMMMMMMMMMMMMMMMMMd
     ;KMMMMMMMWXXWMMMMMMMk.
       "cooc*"    "*coo'"
```

- Terminal: **iterm2**; `tailscale` package installed (no service).
- Rebuild: `darwin-rebuild switch --flake .#meerkat`.

## meerkat — asahi

```
          ▗▄▄▄       ▗▄▄▄▄    ▄▄▄▖            jmoore@meerkat
          ▜███▙       ▜███▙  ▟███▛            --------------
           ▜███▙       ▜███▙▟███▛             OS: NixOS aarch64 (Asahi)
            ▜███▙       ▜██████▛              Host: MacBook Pro (14-inch, M2 Max, 2023)
     ▟█████████████████▙ ▜████▛     ▟▙        Kernel: asahi
    ▟███████████████████▙ ▜███▙    ▟██▙       DE: Hyprland (Wayland)
           ▄▄▄▄▖           ▜███▙  ▟███▛       Terminal: ghostty
          ▟███▛             ▜██▛ ▟███▛        Shell: zsh
         ▟███▛               ▜▛ ▟███▛
▟███████████▛                  ▟██████████▙
▜██████████▛                  ▟███████████▛
      ▟███▛ ▟▙               ▟███▛
     ▟███▛ ▟██▙             ▟███▛
    ▟███▛  ▜███▙           ▝▀▀▀▀
    ▜██▛    ▜███▙ ▜██████████████████▛
     ▜▛     ▟████▙ ▜████████████████▛
           ▟██████▙       ▜███▙
          ▟███▛▜███▙       ▜███▙
         ▟███▛  ▜███▙       ▜███▙
         ▝▀▀▀    ▀▀▀▀▘       ▀▀▀▘
```

- Uses the **pinned** `nixos-apple-silicon` nixpkgs (25.11) and a dedicated,
  era-matched `home-manager-asahi` input (its lib predates `lib.genAttrs'`).
- `peripheralFirmwareHash` must be set (see the abort message if unset).
- HiDPI/retina overrides and a swaylock-instead-of-hyprlock workaround live in
  `asahi.home`; brave/obs/orca-slicer in the asahi config.
- Rebuild on the device: `sudo nixos-rebuild switch --flake .#meerkat`.
