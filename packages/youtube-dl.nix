# Build latest commit of youtube-dl from source because the latest versions in nixpkgs never work
{ pkgs, ... }:

with builtins;
with pkgs;

stdenvNoCC.mkDerivation {
  pname = "youtube-dl";
  version = "unstable-d1c6c5c";

  src = fetchFromGitHub {
    owner = "ytdl-org";
    repo = "youtube-dl";
    rev = "d1c6c5c4d618fa950813c0c71aede34a5ac851e9";
    hash = "sha256-KzsA7SzA9Ye5NRN4eZZ6nzBlqireuF1wFd+AI7/Ifvg=";
  };

  buildInputs = [ python3 pandoc zip ];

  buildPhase = ''
    runHook preBuild
    make
    python3 setup.py build
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    install -Dt $out/bin ./youtube-dl
    runHook postInstall
  '';
}
