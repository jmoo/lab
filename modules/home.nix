{ ... }:
{
  imports = [
    ./apps.nix
    ./direnv.nix
    ./ghostty.nix
    ./hyprland
    ./iterm2
    ./iwmenu.nix
    ./lab.nix
    ./karabiner.nix
    ./shell.nix
    ./theme.nix
    ./vscode.nix
    ./ulauncher.nix
    ./waybar
    ../pkgs/vscode-nix-extensions/home-manager.nix
  ];
}
