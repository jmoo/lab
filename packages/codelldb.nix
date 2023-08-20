{ pkgs, ... }:

with builtins;
with pkgs;

stdenvNoCC.mkDerivation {
  pname = "codelldb-aarch64-darwin.vsix";
  version = "1.9.2";

  src = pkgs.fetchurl {
  	url = "https://github.com/vadimcn/codelldb/releases/download/v1.9.2/codelldb-aarch64-darwin.vsix";
    hash = "sha256-XbJcCxJ3eV4hlqkRijjmmEtHh6wsHj463+7+U3KW/FE=";
  };

  nativeBuildInputs = [ unzip ];

  unpackPhase = ''
    unzip $src
  '';

  installPhase = ''
  	mkdir -p $out
  	cp -r ./extension/* $out
  '';
}
