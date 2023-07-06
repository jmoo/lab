# nixpkgs does not have macos binaries for sublime text
{ pkgs, ... }:

with builtins;
with pkgs;

if stdenv.isDarwin then
  stdenvNoCC.mkDerivation {
    pname = "sublime-merge";
    version = "2085";

    src = fetchzip {
      url = "https://download.sublimetext.com/sublime_merge_build_2085_mac.zip";
      sha256 = "sha256-bcPdIhAIDYV2PPGWt+GaaI20U5LcwJ8mQ/DuHmbIERM=";
    };

    nativeBuildInputs = [ makeWrapper ];

    installPhase = ''
      runHook preInstall
      mkdir -p $out/Applications/Sublime\ Merge.app
      cp -r . $out/Applications/Sublime\ Merge.app
      runHook postInstall
    '';
  }
else
  sublime-merge
