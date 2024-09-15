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
            imports = [ ./module.nix ];
            options = {
              package = mkOption { type = types.package; };
            };
            config = {
              inherit name;
              package = pkgs.callPackage ./default.nix { vscodeExtensionConfig = config; };
            };
          }
        )
      );
      default = { };
    };
  };

  config = {
    programs.vscode.extensions = mapAttrsToList (
      _: cfg: cfg.package
    ) config.programs.vscode.nixExtensions;
  };
}
