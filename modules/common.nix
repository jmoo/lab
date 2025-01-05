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
    "hyprland"
    "iterm2"
    "karabiner"
    "nuphy75"
    "pass"
    "shell"
    "vscode"
  ];

  mkHome =
    x:
    mkMerge [
      (x)
      config.home-manager.common
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

        root = mkEnableOption "Include root user in default users";

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
    _module.args = {
      inherit mkHome;
    };

    home-manager = {
      useGlobalPkgs = true;
      useUserPackages = true;

      # users = foldl' (
      #   x: acc:
      #   acc
      #   // {
      #     "${x}" = mkHome { };
      #   }
      # ) { } (config.lab.users);

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
