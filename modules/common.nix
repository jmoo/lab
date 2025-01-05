{
  inputs,
  lib,
  config,
  ...
}:
with lib;
let
  passthru = [
    "direnv"
    "guake"
    "hyprland"
    "iterm2"
    "karabiner"
    "nuphy75"
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
        enable = mkEnableOption "Enable ${x} configuration for all home-manager users";

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
      };

    home-manager = {
      common = mkOption {
        type = types.deferredModule;
        default = { };
      };
    };
  };

  config = {
    _module.args = {
      mkHome =
        x:
        mkMerge [
          (x)
          config.home-manager.common
        ];
    };

    home-manager = {
      useGlobalPkgs = true;
      useUserPackages = true;

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
                  { enable = mkDefault v.enable; }
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
