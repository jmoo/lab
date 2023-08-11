{ pkgs, ... }:

with builtins;

let
  name = "jmoo-vscode";
  publisher = "jmoo";
  version = "0.0.0";

  vsix = pkgs.stdenvNoCC.mkDerivation {
    inherit version;

    pname = "${publisher}-${name}.vsix";

    src = ./.;

    installPhase = ''
      mkdir -p $out
      cp -r ./* $out/
    '';
  };
in pkgs.vscode-utils.buildVscodeMarketplaceExtension {
  inherit vsix;

  mktplcRef = { inherit name version publisher; };
}
