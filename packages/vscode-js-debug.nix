{ pkgs, ... }:

with builtins;
with pkgs;

stdenvNoCC.mkDerivation {
  pname = "vscode-js-debug.vsix";
  version = "1.80.0";

  src = pkgs.fetchurl {
  	url = "https://github.com/daveleroy/vscode-js-debug/releases/download/v1.80.0/vscode-js-debug.vsix";
    hash = "sha256-s32eBovMzhS50A3tsPXsa95ANQvhpMeyDiufse6sQKs=";
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


