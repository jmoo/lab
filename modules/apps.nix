{
  pkgs,
  config,
  lib,
  ...
}:
with lib;
{
  options.lab.apps = mkOption {
    description = "Default apps";
    type =
      with types;
      attrsOf (
        submodule (
          { name, config, ... }:
          {
            options = {
              enable = (mkEnableOption "Enable default app") // {
                default = true;
              };

              name = mkOption {
                type = types.str;
                default = name;
              };

              package = mkOption {
                type = with types; nullOr package;
                default = null;
              };

              command = mkOption {
                type = types.str;
                default = getExe config.package;
              };
            };
          }
        )
      );
    default = { };
  };

  config.lab.apps = with pkgs; {
    terminal.package = mkDefault xterm;
    bluetoothManager.package = mkDefault blueberry;
    audioManager.package = mkDefault pavucontrol;
    launcher.package = mkDefault config.lab.ulauncher.package;
    displayManager.package = mkDefault wdisplays;
    fileManager.package = mkDefault nemo;
  };
}
