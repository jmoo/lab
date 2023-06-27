# nixpkgs does not have macos binaries for sublime text
{ pkgs, ... }:

with builtins;
with pkgs;

if
  stdenv.isDarwin
then
  stdenvNoCC.mkDerivation {
    pname = "sublime-text";
    version = "4143";

    src = fetchzip {
      url = "https://download.sublimetext.com/sublime_text_build_4143_mac.zip";
      sha256 = "sha256-IyePV9I3JCQOP2VAYsBwpzP26wLcdKEtjieqgeyFlk4=";
    };

    nativeBuildInputs = [ makeWrapper zip ];

    installPhase = ''
      runHook preInstall
      mkdir -p $out/Applications/Sublime\ Text.app
      cp -r . $out/Applications/Sublime\ Text.app
      runHook postInstall
    '';
  }
else
  sublime4
