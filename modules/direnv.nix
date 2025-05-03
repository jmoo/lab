{
  pkgs,
  lib,
  config,
  ...
}:
with lib;
{
  options.lab.direnv.enable = mkEnableOption "Enable direnv home-manager configuration";

  config = mkIf config.lab.direnv.enable {
    programs = {
      direnv = {
        enable = true;
        enableZshIntegration = false;
        enableBashIntegration = false;
        enableNushellIntegration = false;
        enableFishIntegration = false;
        nix-direnv.enable = false;
      };

      vscode.profiles.default.extensions = with pkgs.vscode-extensions; [ mkhl.direnv ];
    };
  };
}
