{ pkgs, ... }:

with builtins;
with pkgs;

stdenvNoCC.mkDerivation {
  pname = "LSP-typescript.sublime-package";
  version = "2.4.0";

  src = pkgs.fetchFromGitHub {
    owner = "sublimelsp";
    repo = "LSP-typescript";
    rev = "e6d67aa95aca1013b0080c64458fd23fe0ba8cf4";
    hash = "sha256-uBwKYxurXJw97VLZZu6mqCtum8mtE2XeUFIVNfKDc+k=";
  };

  nativeBuildInputs = [ zip ];

  buildPhase = "	zip -r $out ./\n";
}
