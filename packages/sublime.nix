# nixpkgs does not have macos binaries for sublime text
{ pkgs, ... }:

with builtins;
with pkgs;

if stdenv.isDarwin then
  stdenvNoCC.mkDerivation {
    pname = "sublime-text";
    version = "4150";

    src = fetchzip {
      url = "https://download.sublimetext.com/sublime_text_build_4150_mac.zip";
      sha256 = "sha256-7Wkxc7FuQpRfVFpD2nwRo10ftQjfQ6aQVSO1F5WC2CE=";
    };

    nativeBuildInputs = [ makeWrapper ];

    installPhase = ''
      runHook preInstall
      mkdir -p $out/Applications/Sublime\ Text.app
      cp -r . $out/Applications/Sublime\ Text.app

      mkdir -p $out/bin

      makeWrapper "$out/Applications/Sublime Text.app/Contents/MacOS/sublime_text" "$out/bin/sublime_text"

      runHook postInstall
    '';
  }
else
  sublime4
