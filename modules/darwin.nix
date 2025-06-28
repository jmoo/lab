{
  inputs,
  pkgs,
  config,
  lib,
  ...
}:
with lib;
{
  imports = [
    inputs.home-manager.darwinModules.home-manager
    ./common.nix
  ];

  config = mkMerge [
    {
      lab.common = {
        home.stateVersion = mkDefault "25.05";
        programs.home-manager.enable = false;
      };

      users.users.root.home = "/var/root";
      nix.enable = true;
      system.stateVersion = mkDefault 5;
      nixpkgs.hostPlatform = mkDefault "aarch64-darwin";
    }

    (mkIf config.lab.iterm2.enable {
      environment.systemPackages = with pkgs; [ iterm2 ];
    })

    (mkIf config.lab.shell.enable {
      lab.common.home.shellAliases = {
        switch = mkDefault "sudo darwin-rebuild switch --flake ${config.lab.source}#${config.lab.name}";
      };
      programs.zsh.enable = true;
    })
  ];
}
