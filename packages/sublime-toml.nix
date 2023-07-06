{ pkgs, ... }:

with builtins;
with pkgs;

stdenvNoCC.mkDerivation {
  pname = "TOML.sublime-package";
  version = "2.5.0";

  src = pkgs.fetchFromGitHub {
    owner = "jasonwilliams";
    repo = "sublime_toml_highlighting";
    rev = "ed38438900d6b128353cd1aa1364e2e3b8ffb8a2";
    hash = "sha256-3A7mpQtsiXHh3Z5KfnfKQF3jYzjUr0uGJ2XOuVrs7As=";
  };

  nativeBuildInputs = [ zip ];

  buildPhase = "	zip -r $out ./\n";
}
