{ pkgs, lib, ... }:
with lib;
{
  imports = [
    ./direnv.nix
    ./guake.nix
    ./iterm2.nix
    ./karabiner.nix
    ./nuphy75.nix
    ./shell.nix
    ./vscode.nix
    ../pkgs/vscode-nix-extensions/home-manager.nix
  ];

  programs.home-manager.enable = mkDefault true;

  home.stateVersion = mkDefault "24.05";
}
