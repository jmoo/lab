{ pkgs, ... }:

with builtins;
with pkgs;

stdenvNoCC.mkDerivation {
  pname = "LSP-bash.sublime-package";
  version = "2.0.24";

  src = pkgs.fetchFromGitHub {
    owner = "sublimelsp";
    repo = "LSP-bash";
    rev = "2f464910a39856f8e775c73b805387533ab7d79e";
    hash = "sha256-pgVTLvmZPde2hwJpU4hsSzody6pnyannW/OjputW4yk=";
  };

  nativeBuildInputs = [ zip ];

  buildPhase = "	zip -r $out ./\n";
}
