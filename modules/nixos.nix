{
  config,
  lib,
  pkgs,
  ...
}:
with lib;
{
  imports = [
    ./home-manager.nix
    ./k3s.nix
    ./nix.nix
  ];

  config = mkMerge [
    {
      i18n.defaultLocale = "en_US.UTF-8";
      i18n.extraLocaleSettings = {
        LC_ADDRESS = "en_US.UTF-8";
        LC_IDENTIFICATION = "en_US.UTF-8";
        LC_MEASUREMENT = "en_US.UTF-8";
        LC_MONETARY = "en_US.UTF-8";
        LC_NAME = "en_US.UTF-8";
        LC_NUMERIC = "en_US.UTF-8";
        LC_PAPER = "en_US.UTF-8";
        LC_TELEPHONE = "en_US.UTF-8";
        LC_TIME = "en_US.UTF-8";
      };

      system.stateVersion = mkDefault "25.05";
      time.timeZone = "America/New_York";
    }

    (mkIf config.lab.hyprland.enable {
      environment.systemPackages = with pkgs; [
        kitty
      ];

      home-manager.common.lab.hyprland.enable = mkDefault true;
      programs.hyprland.enable = true;
    })

    (mkIf config.lab.shell.enable {
      home-manager.common.lab.shell.enable = mkDefault true;
      users.defaultUserShell = pkgs.zsh;
      programs.zsh.enable = true;
      programs.fzf.fuzzyCompletion = true;
    })
  ];
}
