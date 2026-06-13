{
  pkgs,
  lib,
  config,
  ...
}:
with lib;
{
  options = {
    programs.vscode.nixExtensions = mkOption {
      type = types.attrsOf (
        types.submodule (
          { config, name, ... }:
          {
            config = {
              inherit name;
              package = pkgs.callPackage ./default.nix { vscodeExtensionConfig = config; };
            };
            imports = [ ./module.nix ];
            options = {
              package = mkOption { type = types.package; };
            };
          }
        )
      );
      default = { };
    };
  };

  config = {
    programs.vscode.profiles.default.extensions = mapAttrsToList (
      _: cfg: cfg.package
    ) config.programs.vscode.nixExtensions;
  };
}
