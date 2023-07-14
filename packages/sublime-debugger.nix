{ pkgs, ... }:

with builtins;
with pkgs;

stdenvNoCC.mkDerivation {
  pname = "SublimeDebugger.sublime-package";
  version = "0.10.1";

  src = pkgs.fetchFromGitHub {
    owner = "daveleroy";
    repo = "SublimeDebugger";
    rev = "7c88fa6402e154f41e4db1075bd336bc539b035f";
    hash = "sha256-9VH3lZ4bO6msF1zTU8irQz7YuqBVO7PatKezaP9Ol9s=";
  };

  nativeBuildInputs = [ zip ];

  buildPhase = "	zip -r $out ./\n";
}
