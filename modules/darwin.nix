{
  inputs,
  pkgs,
  config,
  lib,
  mkHome,
  ...
}:
with lib;
{
  imports = [
    inputs.home-manager.darwinModules.home-manager
    ./home-manager.nix
    ./nix.nix
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

  config = mkMerge [
    {
      home-manager = {
        common = {
          home.stateVersion = mkDefault "25.05";
          programs.home-manager.enable = false;
        };

        users.root = mkHome { };
      };

      users.users.root.home = "/var/root";
      system.stateVersion = 5;
      services.nix-daemon.enable = true;
      nixpkgs.hostPlatform = mkDefault "aarch64-darwin";
    }

    (mkIf config.lab.iterm2.enable {
      environment.systemPackages = with pkgs; [ iterm2 ];
    })

    (mkIf config.lab.shell.enable {
      home-manager.common.home.shellAliases = {
        switch = mkDefault "darwin-rebuild switch --flake ${config.lab.source}#${config.lab.name}";
      };
      programs.zsh.enable = true;
    })
  ];
}
