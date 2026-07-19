inputs: final: prev:
let
  lib' = prev.lib.extend (import ./lib.nix inputs);
in
{
  # # Fix core dump on asahi
  # # This PR gets a little farther but still segfaults
  # hyprlock = prev.hyprlock.overrideAttrs (_: {
  #   src = final.fetchFromGitHub {
  #     owner = "jaakkomoller";
  #     repo = "hyprlock";
  #     rev = "839";
  #     hash = "sha256-raYdkw32pEE9HetrIu7jHOWiSmp8YTBLMPVekV46+I4=";
  #   };
  # });

  # Private repository of clavia nord files to test nord-format against
  nord-corpus = builtins.fetchGit {
    rev = "b80431bcccddbb07bf5bcccb7dce42968c404898";
    url = "git+ssh://git@github.com/jmoo/nord-corpus.git";
  };

  nudelta = inputs.nudelta.packages.${prev.stdenv.hostPlatform.system}.default;

  open-bamboo-networking = final.callPackage ./pkgs/open-bamboo-networking { };

  ulauncher-uwsm = final.callPackage ./pkgs/ulauncher-uwsm { };

  vscode-extensions = prev.vscode-extensions // {
    mkVscodeNixExtension =
      config:
      final.vscode-extensions.vscode-nix-extensions.override {
        vscodeExtensionModule = config;
      };

    vscode-nix-extensions = final.callPackage ./pkgs/vscode-nix-extensions { };
  };
}
// lib'.lab.mkScripts final ./scripts
// lib'.lab.mkRustCrates final ./crates
