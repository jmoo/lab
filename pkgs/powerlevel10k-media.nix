# Adapted from https://github.com/NixOS/nixpkgs/blob/nixos-unstable/pkgs/data/fonts/powerline-fonts/default.nix
{ lib, pkgs }:

with builtins;

pkgs.stdenvNoCC.mkDerivation {
  pname = "powerlevel10k-media";
  version = "unstable-389133f";

  src = pkgs.fetchFromGitHub {
    owner = "romkatv";
    repo = "powerlevel10k-media";
    rev = "389133fb8c9a2347929a23702ce3039aacc46c3d";
    hash = "sha256-GGfON6Z/0czCUAGxnqtndgDnaZGONFTU9/Hu4BGDHlk=";
  };

  installPhase = ''
    runHook preInstall
    find . -name '*.otf'    -exec install -Dt $out/share/fonts/opentype {} \;
    find . -name '*.ttf'    -exec install -Dt $out/share/fonts/truetype {} \;
    find . -name '*.bdf'    -exec install -Dt $out/share/fonts/bdf      {} \;
    find . -name '*.pcf.gz' -exec install -Dt $out/share/fonts/pcf      {} \;
    find . -name '*.psf.gz' -exec install -Dt $out/share/consolefonts   {} \;
    runHook postInstall
  '';
}
