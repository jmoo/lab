{
  lib,
  config,
  ...
}:
with lib;
let
  # Home-manager modules that can be configured for multiple users via NixOS or nix-darwin
  passthru = [
    "direnv"
    "ghostty"
    "hyprland"
    "hyprpaper"
    "hypridle"
    "hyprlock"
    "iterm2"
    "karabiner"
    "theme"
    "shell"
    "vscode"
    "waybar"
  ];
in
{
  imports = [
    ./lab.nix
    ./nix.nix
  ];

  options = {
    lab =
      (genAttrs passthru (x: {
        enable = mkEnableOption "Enable ${x} home-manager configuration for all users";

        users = mkOption {
          type = with types; listOf str;
          default = config.lab.users;
        };

        root = mkEnableOption "Apply ${x} home-manager configuration for the root user";

        common = mkOption {
          description = "Common ${x} configuration for all home-manager users";
          type = types.raw // {
            merge = _: defs: mkMerge (map (x: x.value) defs);
          };
          default = { };
        };
      }))
      // {
        name = mkOption {
          default = config.networking.hostName;
        };

        users = mkOption {
          description = "Users that home-manager configuration will be applied to (excluding root)";
          type = with types; listOf str;
          default = [ ];
        };

        root = mkEnableOption "Apply home-manager configuration to the root user";

        common = mkOption {
          description = "Common configuration for all home-manager users";
          type = types.deferredModule;
          default = { };
        };
      };
  };

  config = {
    home-manager = {
      useGlobalPkgs = true;
      useUserPackages = true;
      users = foldl' (
        acc: x:
        acc
        // {
          "${x}" = _: config.lab.common;
        }
      ) { } (config.lab.users ++ (lists.optional config.lab.root "root"));
    };

    lab.common =
      { name, ... }:
      {
        imports = [
          ./home.nix
        ];

        lab =
          (filterAttrs (n: _: elem n passthru) (
            mapAttrs (
              n: v:
              mkMerge [
                { enable = mkIf (v.enable && (elem name v.users || (v.root && name == "root"))) true; }
                v.common
              ]
            ) config.lab
          ))
          // {
            name = mkDefault config.lab.name;
            source = mkDefault config.lab.source;
          };

        home = {
          homeDirectory = mkDefault config.users.users.${name}.home;
          username = mkDefault config.users.users.${name}.name;
        };

        programs.home-manager.enable = false;
      };
  };
}
