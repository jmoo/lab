inputs: final: prev: {
  vscode-extensions = prev.vscode-extensions // {
    mkExtension = config:
      final.vscode-extensions.vscode-extend.override {
        vscodeExtensionModule = config;
      };

    nix-vscode-extend =
      final.callPackage ./pkgs/nix-vscode-extend/package.nix { };

    vscode-extend = final.callPackage ./pkgs/vscode-extend { };
  };
}
