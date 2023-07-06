{ pkgs, ... }:

with builtins;
with pkgs;

stdenvNoCC.mkDerivation {
  pname = "Nix.sublime-package";
  version = "2.3.2";

  src = pkgs.fetchFromGitHub {
    owner = "wmertens";
    repo = "sublime-nix";
    rev = "ebf06571f3cc2b30a7f88bbd4eb371b72ce828c2";
    hash = "sha256-1FfqlhPF5X+qwPxsw7ktyHKgH6VMKk0PV+LIXrGtbt4=";
  };

  nativeBuildInputs = [ zip ];

  buildPhase = "	zip -r $out ./\n";
}
