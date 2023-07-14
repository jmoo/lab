{ pkgs, ... }:

with builtins;
with pkgs;

stdenvNoCC.mkDerivation {
  pname = "vscode-python-debug.vsix";
  version = "1.80.0";

  src = pkgs.fetchurl {
  	url = "https://github.com/daveleroy/vscode-python/releases/download/2023.8.0/vscode-python.vsix";
    hash = "sha256-JmWHH48bP8+LF8TfVrcNcdOhR2XI2zZXwi8t+uaBNzw=";
  };

  nativeBuildInputs = [ unzip ];

  unpackPhase = ''
    unzip $src
  '';

  installPhase = ''
  	mkdir -p $out
  	cp -r ./* $out
  '';
}


