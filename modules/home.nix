{
  pkgs,
  lib,
  ...
}:
with lib;
{
  imports = [
    ./direnv.nix
    ./guake.nix
    ./hyprland
    ./iterm2.nix
    ./lab.nix
    ./karabiner.nix
    ./nuphy75.nix
    ./pass.nix
    ./shell.nix
    ./vscode.nix
    ./ulauncher.nix
    ./waybar
    ../pkgs/vscode-nix-extensions/home-manager.nix
  ];

  config = {
    programs.home-manager.enable = mkDefault true;
  };
}
