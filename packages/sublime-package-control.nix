{ pkgs, ... }:

with builtins;
with pkgs;

stdenvNoCC.mkDerivation {
  pname = "Package Control.sublime-package";
  version = "3.4.1";

  src = pkgs.fetchFromGitHub {
    owner = "wbond";
    repo = "package_control";
    rev = "8b947d227bfee2b514283e650c3f88c954ae1026";
    hash = "sha256-25lw2qGy/88pIq+RUzoyAtwIU7mhVPgmQiq6OTlIx4M=";
  };

  nativeBuildInputs = [ zip ];

  buildPhase = "	zip -r $out ./\n";
}
