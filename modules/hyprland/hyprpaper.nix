{
  config,
  lib,
  pkgs,
  ...
}:
with lib;
let
  wallpapers = filter (
    x:
    x.enable
    && elem x.type [
      "desktop"
      "lock_and_desktop"
    ]
  ) config.lab.hyprland.wallpapers;

  static = filter (x: x.monitors != null) wallpapers;
  dynamic = filter (x: x.monitors == null) wallpapers;
  preload = filter (x: x.preload) wallpapers;
  monitors = concatLists (map (x: if x.monitors == null then [ ] else x.monitors) wallpapers);
  source = x: "${optionalString (x.mode != "cover") "${x.mode}:"}${toString x.source}";
in
{
  options.lab.hyprpaper = {
    enable = mkEnableOption "Enable hyprpaper for wallpaper management in home-manager";
  };

  config = mkIf config.lab.hyprpaper.enable (mkMerge [
    {
      services.hyprpaper = {
        enable = true;

        settings = {
          ipc = "on";
          splash = false;
          preload = map (x: toString x.source) preload;
          wallpapers =
            (map (x: " , ${source x}") dynamic)
            ++ (concatLists (map (x: map (m: "${m}, ${source x}") x.monitors) static));
        };
      };

      # Hyprpaper doesn't currently set dynamic wallpapers correctly,
      # so we will set them at startup. We should be able to delete this
      # eventually because the hyprpaper docs explicitly state this is
      # possible despite it not working on this version.
      systemd.user.services.hyprpaper.Service.ExecStartPost = pkgs.writeScript "hyprpaper-apply" ''
        #!/usr/bin/env bash
        set -euxo pipefail
        sleep 2
        monitors=(${concatStringsSep " " (map (x: ''"${source x}"'') monitors)})
        wallpapers=(${concatStringsSep " " (map (x: ''"${source x}"'') dynamic)})
        selected=''${wallpapers[ $RANDOM % ''${#wallpapers[@]} ]}

        echo "Monitors: ''${monitors[*]}"
        echo "Wallpapers: ''${wallpapers[*]}"
        echo "Selected: ''${selected}"

        for monitor in $(hyprctl monitors | grep 'Monitor' | awk '{ print $2 }'); do
          if [[ ! " ''${monitors[*]} " =~ [[:space:]]''${monitor}[[:space:]] ]]; then
              echo "Setting wallpaper \"$selected\" for monitor \"$monitor\""
              hyprctl hyprpaper wallpaper "$monitor,$selected"
          else
            echo "Wallpaper already set for monitor \"$monitor\""
          fi
        done
      '';

      wayland.windowManager.hyprland.settings = {
        # Disable default anime wallpapers
        misc = {
          force_default_wallpaper = 0;
          disable_hyprland_logo = true;
        };
      };
    }
  ]);
}
