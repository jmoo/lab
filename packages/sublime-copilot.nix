{ pkgs, ... }:

with builtins;
with pkgs;

stdenvNoCC.mkDerivation {
  pname = "LSP-copilot.sublime-package";
  version = "0.1.22";

  src = pkgs.fetchFromGitHub {
    owner = "TerminalFi";
    repo = "LSP-copilot";
    rev = "781b217d8de8cd32598d0e8f0be66a1773fe0850";
    hash = "sha256-JAoZIHccEo2MLOXw5jCVwarwiv8nPBkcm1bOB9DmD7Y=";
  };

  nativeBuildInputs = [ zip ];

  buildPhase = "	zip -r $out ./\n";
}
