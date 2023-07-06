{ pkgs, ... }:

with builtins;
with pkgs;

stdenvNoCC.mkDerivation {
  pname = "SideBarEnhancements";
  version = "5.0.49";

  src = pkgs.fetchFromGitHub {
    owner = "titoBouzout";
    repo = "SideBarEnhancements";
    rev = "289fa49d005352c47cfb9ba36656794dade7ced3";
    hash = "sha256-d0YQku6jLGKu4qsK13OdOc2+m/6dDRY7tQaexV7ThLI=";
  };

  nativeBuildInputs = [ zip ];

  buildPhase = "zip -r $out ./\n";
}
