{ lib', ... }:
let
  inherit (lib'.lab) mkHostModule homeLinux hyprVar;
  inherit (lib') mkEnableOption mkIf mkMerge;
in
{
  options.lab.hosts = mkHostModule (
    { config, ... }:
    {
      config = mkIf config.hyprlock.enable (mkMerge [
        (homeLinux {
          programs.hyprlock = {
            enable = true;
            extraConfig = builtins.readFile ./hyprlock.conf;
          };
        })

        # `lock` keybind in the hyprland config.
        (hyprVar "lock" (
          {
            pkgs,
            lib,
            config,
            ...
          }:
          "${lib.getExe pkgs.uwsm} app -- ${lib.getExe config.programs.hyprlock.package}"
        ))
      ]);

      options.hyprlock.enable = mkEnableOption "hyprlock lock screen";
    }
  );
}
