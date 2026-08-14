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

    # Bump claude-code past nixpkgs' 2.1.158 so Opus 5 (claude-opus-5) is
    # available — the model landed in 2.1.219; below that `opus` still resolves
    # to Opus 4.8 and Opus 5 never shows in the picker. Same native-binary
    # derivation as nixpkgs, just re-pointed at a vendored (newer) manifest.
    claude-code =
      let
        manifest = builtins.fromJSON (builtins.readFile ./pkgs/claude-code/manifest.json);
        # Derived from portable `is*` predicates rather than
        # `hostPlatform.node` — the pinned Asahi nixpkgs lib predates `node`.
        inherit (final.stdenv) hostPlatform;
        platformKey = "${
          if hostPlatform.isDarwin then
            "darwin"
          else if hostPlatform.isWindows then
            "win32"
          else
            "linux"
        }-${if hostPlatform.isAarch64 then "arm64" else "x64"}";
      in
      prev.claude-code.overrideAttrs (old: {
        inherit (manifest) version;
        passthru = old.passthru // {
          updateScript = ./pkgs/claude-code/update.sh;
        };
        src = final.fetchurl {
          url = "https://storage.googleapis.com/claude-code-dist-86c565f3-f756-42ad-8dfa-d59b1c096819/claude-code-releases/${manifest.version}/${platformKey}/claude";
          sha256 = manifest.platforms.${platformKey}.checksum;
        };
      });

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
  })
]
