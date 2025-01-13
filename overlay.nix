inputs: final: prev: {
  blueberry = prev.blueberry.overrideAttrs (old: {
    meta = old.meta // {
      mainProgram = "blueberry";
    };
  });

  breeze2-sddm-theme = final.stdenvNoCC.mkDerivation {
    name = "breeze2-sddm-theme";
    src = final.fetchFromGitHub {
      owner = "avivace";
      repo = "breeze2-sddm-theme";
      rev = "8087fa6f9826f51a766c48b59ce1c06c323a3540";
      hash = "sha256-CrVoE96mDjIjU+5U2rJnqXv7D7Mft+9CfV7INsFBS74=";
    };
    phases = [ "installPhase" ];
    installPhase = ''
      mkdir -p $out/share/sddm/themes/breeze
      cp -R $src/* $out/share/sddm/themes/breeze/
    '';
  };

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
