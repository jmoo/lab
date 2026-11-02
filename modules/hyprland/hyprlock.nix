{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeLinux;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      config = mkIf config.hyprlock.enable (
        homeLinux (
          {
            pkgs,
            lib,
            config,
            ...
          }:
          {
            programs.hyprlock = {
              enable = true;
              extraConfig = builtins.readFile ./hyprlock.conf;
            };

            # `lock` keybind in hyprland.lua.
            wayland.windowManager.hyprland.settings.lock._var =
              "${lib.getExe pkgs.uwsm} app -- ${lib.getExe config.programs.hyprlock.package}";
          }
        )
      );

      options.hyprlock.enable = mkEnableOption "hyprlock lock screen";
    }
  );
}
