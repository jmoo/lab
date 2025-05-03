inputs: final: prev: {
  blueberry = prev.blueberry.overrideAttrs (old: {
    meta = old.meta // {
      mainProgram = "blueberry";
    };
  });

  iwmenu = inputs.iwmenu.packages.${final.system}.default;

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
