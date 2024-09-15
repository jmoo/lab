inputs: final: prev: {
  powerlevel10k-media = final.callPackage ./pkgs/powerlevel10k-media.nix { };

  vscode-extensions = prev.vscode-extensions // {
    mkVscodeNixExtension = config:
      final.vscode-extensions.vscode-nix-extensions.override {
        vscodeExtensionModule = config;
      };

    vscode-nix-extensions = final.callPackage ./pkgs/vscode-nix-extensions { };
  };
}
