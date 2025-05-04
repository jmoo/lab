inputs: final: prev: {
  blueberry = prev.blueberry.overrideAttrs (old: {
    meta = old.meta // {
      mainProgram = "blueberry";
    };
  });

  device-tree-xlnx = final.fetchFromGitHub {
    owner = "Xilinx";
    repo = "device-tree-xlnx";
    rev = "xlnx_rel_v2024.1";
    sha256 = "sha256-dja+JwbXwiBRJwg/6GNOdONp/vrihmfPBnpjEA/xxnk=";
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

  nixos-xlnx = final.callPackage ./pkgs/nixos-xlnx {
    inherit inputs;
  };
}
