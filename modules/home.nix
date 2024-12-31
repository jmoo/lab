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
    ./hyprland/home.nix
    ./iterm2.nix
    ./karabiner.nix
    ./nuphy75.nix
    ./pass.nix
    ./shell.nix
    ./vscode.nix
    ../pkgs/vscode-nix-extensions/home-manager.nix
  ];

  options = {
    lab = {
      name = mkOption {
        type = types.str;
        default = config.networking.hostName;
      };

      source = mkOption {
        type = with types; nullOr str;
        default = "github:jmoo/lab";
      };
    };
  };

  config = {
    programs.home-manager.enable = mkDefault true;
  };
}
