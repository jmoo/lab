{
  config,
  lib,
  ...
}:
with lib;
{
  options.lab.ghostty = {
    enable = mkEnableOption "Enable ghostty home-manager configuration";
  };

  config = mkIf config.lab.ulauncher.enable {
    lab.apps.terminal.package = config.programs.ghostty.package;

    programs.ghostty = {
      enable = true;
    };
  };
}
