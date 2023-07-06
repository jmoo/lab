{ pkgs, ... }:

with builtins;
with pkgs;

stdenvNoCC.mkDerivation {
  pname = "LSP.sublime-package";
  version = "4070-1.24.0";

  src = pkgs.fetchFromGitHub {
    owner = "sublimelsp";
    repo = "LSP";
    rev = "fb623c4bebe69411ca3c2c436cacfd23fb26b969";
    hash = "sha256-RlGLsdhT5P0k4C0CA2OOazfbRYLoe8BZjz9Ycdp/26w=";
  };

  nativeBuildInputs = [ zip ];

  buildPhase = "	zip -r $out ./\n";
}
