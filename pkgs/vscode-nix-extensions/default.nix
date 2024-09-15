{ pkgs, stdenv, vscode-utils, writeText, vscodeExtensionModule ? { }
, vscodeExtensionConfig ? null, ... }:
with pkgs.lib;
let
  config = if vscodeExtensionConfig != null then
    vscodeExtensionConfig
  else
    (evalModules {
      modules = [
        ./module.nix
        { _module.args = { inherit pkgs; }; }
        vscodeExtensionModule
      ];
    }).config;

  packageJson = writeText "pacakge.json" (builtins.toJSON config.extraConfig);

in (vscode-utils.buildVscodeMarketplaceExtension {
  mktplcRef = { inherit (config) name version publisher; };
  sourceRoot = null;
  vsix = stdenv.mkDerivation {
    name = "${config.name}.vsix";
    version = config.version;
    src = ./src;
    phases = [ "installPhase" ];
    installPhase = ''
      mkdir -p $out/nix/store
      cp -R $src/extension.js $out/
      cp ${packageJson} $out/package.json

      # Add debug/launch configuration for extension
      ${optionalString config.debug ''
        mkdir -p $out/.vscode
        cp $src/launch.json $out/.vscode/launch.json
      ''}

      # Link paths that vscode requires to be local
      ${concatStringsSep "\n" (map (x:
        if isAttrs x then ''
          ln -s ${x.from} $out/${x.to}
        '' else ''
          ln -s ${x} $out${x}
        '') config.paths)}
    '';
  };
}).overrideAttrs { sourceRoot = null; }
