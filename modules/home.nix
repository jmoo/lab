{ pkgs, lib, ... }:
with lib; {
  imports =
    [ ./iterm2.nix ./shell.nix ./vscode.nix ../pkgs/vscode-nix-extensions/home-manager.nix ];

  programs.home-manager.enable = mkDefault true;

  home.stateVersion = mkDefault "24.05";
}
