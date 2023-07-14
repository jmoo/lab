{ pkgs, ... }:

with builtins;
with pkgs;

stdenvNoCC.mkDerivation {
  pname = "LSP-pylsp.sublime-package";
  version = "2.14.0";

  src = pkgs.fetchFromGitHub {
    owner = "sublimelsp";
    repo = "LSP-pylsp";
    rev = "4b1fcd798dfcd148893e308f74deef0250bc2bac";
    hash = "sha256-iwDbcXn9DPKU+EgZD7VcBpOziu0hex7u5hyY72HUNoA=";
  };

  nativeBuildInputs = [ zip ];

  buildPhase = "	zip -r $out ./\n";
}
