{
  lib,
  config,
  ...
}:
with lib;
let
  passthru = [
    "direnv"
    "guake"
    "ghostty"
    "hyprland"
    "hypridle"
    "hyprlock"
    "iterm2"
    "karabiner"
    "nuphy75"
    "theme"
    "pass"
    "shell"
    "vscode"
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
        enable = mkEnableOption "Enable ${x} configuration";

        users = mkOption {
          type = with types; listOf str;
          default = config.lab.users;
        };

        root = mkEnableOption "Enable the root user";

        common = mkOption {
          description = "Common ${x} configuration for all home-manager users";
          type = types.raw;
          default = { };
        };
      }))
      // {
        name = mkOption {
          default = config.networking.hostName;
        };

        users = mkOption {
          type = with types; listOf str;
          default = [ ];
        };

        root = mkEnableOption "Enable the root user";
      };

    home-manager = {
      common = mkOption {
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
          "${x}" = config.home-manager.common;
        }
      ) { } (config.lab.users ++ (lists.optional config.lab.root "root"));

      common =
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
                  { enable = mkDefault (v.enable && (elem name v.users || (v.root && name == "root"))); }
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
  };
}
