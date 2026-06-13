{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeLinux;
  inherit (lib') mkEnableOption mkIf;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      options.hyprlock.enable = mkEnableOption "hyprlock lock screen";

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

            # $lock keybind referenced by hyprland.conf.
            wayland.windowManager.hyprland.settings."$lock" =
              "${lib.getExe pkgs.uwsm} app -- ${lib.getExe config.programs.hyprlock.package}";
          }
        )
      );
    }
  );
}
