{ pkgs, ... }:

with builtins;
with pkgs;

stdenvNoCC.mkDerivation {
  pname = "LSP-rust-analyzer.sublime-package";
  version = "1.5.0";

  src = pkgs.fetchFromGitHub {
    owner = "sublimelsp";
    repo = "LSP-rust-analyzer";
    rev = "91009610080e3b4e65aa1397e0e6f0a79017ac63";
    hash = "sha256-wkuFQ4F33BBVyBAIZl3XGaTZZjBN23RZ1AMIHyELkGI=";
  };

  nativeBuildInputs = [ zip ];

  buildPhase = "	zip -r $out ./\n";
}
