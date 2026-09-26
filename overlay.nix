inputs:
let
  lib' = inputs.nixpkgs.lib.extend (import ./lib.nix inputs);
  inherit (lib'.lab) mkScripts mkRustCrates;
in
lib'.composeManyExtensions [
  (final: _: mkScripts final ./scripts)
  (final: _: mkRustCrates final ./crates)
  (final: prev: {
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

    nudelta = inputs.nudelta.packages.${prev.stdenv.hostPlatform.system}.default;

    open-bamboo-networking = final.callPackage ./pkgs/open-bamboo-networking { };

    # Qwen-Image 2.1 (mflux-generate-qwen-2.1) arrived in mflux 0.20.0; nixpkgs
    # is still on 0.15.4. Tests need network access, so they stay off.
    pythonPackagesExtensions = prev.pythonPackagesExtensions ++ [
      (pyfinal: pyprev: {
        # nixpkgs builds mlx without Metal (Apple's `metal` compiler is not
        # open source, so the kernels cannot be compiled in the sandbox), which
        # leaves every model on the CPU. On darwin use PyPI's prebuilt pair
        # instead: `mlx` is a shim that hard-depends on `mlx-metal`, which
        # carries libmlx + the metallib; they overlap in a few files, so both
        # land in one package. ⚠️ Pinned to the interpreter's cp314 tag.
        mlx =
          if pyprev.stdenv.hostPlatform.isDarwin then
            pyfinal.buildPythonPackage rec {
              format = "wheel";
              inherit (pyprev.mlx) meta;
              metal = pyfinal.fetchPypi {
                abi = "none";
                dist = "py3";
                format = "wheel";
                hash = "sha256-W2SyCsJLDEAfSJ3gHoIJ7cTTchJSAfGTFObznjhTIqo=";
                platform = "macosx_14_0_arm64";
                pname = "mlx_metal";
                python = "py3";
                inherit version;
              };
              nativeBuildInputs = [ final.unzip ];
              pname = "mlx";
              postInstall = ''
                unzip -n -q $metal -d $out/${pyfinal.python.sitePackages}
              '';
              pythonImportsCheck = [ "mlx.core" ];
              pythonRemoveDeps = [ "mlx-metal" ];
              src = pyfinal.fetchPypi {
                abi = "cp314";
                dist = "cp314";
                format = "wheel";
                hash = "sha256-LuebH4wsKjKa/JXs59zgvnmNQ/PedxpjcNK5+XArvZo=";
                platform = "macosx_14_0_arm64";
                inherit pname version;
                python = "cp314";
              };
              version = "0.32.0";
            }
          else
            pyprev.mlx;

        mflux = pyprev.mflux.overridePythonAttrs (old: rec {
          dependencies = old.dependencies ++ [
            pyfinal.hf-transfer
            pyfinal.protobuf
            pyfinal.pyyaml
          ];
          doCheck = false;
          postPatch = ''
            substituteInPlace pyproject.toml \
              --replace-fail "uv_build>=0.12.5,<0.13.0" "uv_build"
          '';
          src = final.fetchFromGitHub {
            hash = "sha256-lKnGhtEZYxrdTARfZa0sN4zVqDQFLC+2xzVNw1ddmZQ=";
            owner = "filipstrand";
            repo = "mflux";
            tag = "v.${version}";
          };
          version = "0.20.0";
        });
      })
    ];

    ulauncher-uwsm = final.callPackage ./pkgs/ulauncher-uwsm { };

    vscode-extensions = prev.vscode-extensions // {
      mkVscodeNixExtension =
        config:
        final.vscode-extensions.vscode-nix-extensions.override {
          vscodeExtensionModule = config;
        };

      vscode-nix-extensions = final.callPackage ./pkgs/vscode-nix-extensions { };
    };
  })
]
