inputs: final: prev: {
  blueberry = prev.blueberry.overrideAttrs (old: {
    meta = old.meta // {
      mainProgram = "blueberry";
    };
  });

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
