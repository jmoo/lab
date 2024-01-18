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
    rev = "be008e657d79832642e2158557c899249c9e31cd";
    hash = "sha256-iUYWKrJtEQPlFPHkkY6kYyESRbDys6+HRGkiN8Dg1o4=";
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
