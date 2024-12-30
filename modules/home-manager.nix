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
    inputs.home-manager.nixosModules.home-manager
  ];

  options = {
    lab = genAttrs passthru (x: {
      enable = mkEnableOption "Enable ${x} configuration for all home-manager users";
      common = mkOption {
        description = "Common ${x} configuration for all home-manager users";
        type = types.raw;
        default = { };
      };
    });

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

          lab = filterAttrs (n: _: elem n passthru) (
            mapAttrs (
              n: v:
              mkMerge [
                { enable = mkDefault v.enable; }
                v.common
              ]
            ) config.lab
          );

          home = {
            homeDirectory = mkDefault "/home/${name}";
            username = mkDefault name;
            stateVersion = config.system.stateVersion;
          };

          programs.home-manager.enable = false;
        };
    };
  };
}
