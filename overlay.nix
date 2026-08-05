inputs:
let
  lib' = inputs.nixpkgs.lib.extend (import ./lib.nix inputs);
  inherit (lib'.lab) mkScripts mkRustCrates;
in
lib'.composeManyExtensions [
  (final: _: mkScripts final ./scripts)
  (final: _: mkRustCrates final ./crates)
  (
    final: prev:
    let
      # The corpus repository as fetched — deliberately *not* an overlay attribute.
      #
      # ⚠️ The tree carries `library.json`, which is the R2 address of every R2-tier
      # object. This repo is public, so nothing may hand a consumer the raw tree: what
      # lab exposes is the corpus's own assembly, which is the git tier filtered
      # against that index.
      nord-corpus-tree = builtins.fetchGit {
        # ⚠️ The pinned rev lives on `size-tiering`, not the default branch, and
        # `fetchGit` only fetches the refs it is told about — without this it reports
        # the rev as not found. Drop it when the branch merges.
        ref = "size-tiering";
        rev = final.nord-corpus-rev;
        url = "git+ssh://git@github.com/jmoo/nord-corpus.git";
      };
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

      nord-cli = prev.lib.addMetaAttrs { mainProgram = "nord"; } prev.nord-cli;

      # The specimen corpus nord-format and nord-usb are tested against, at the pinned
      # revision: a model directory per instrument, the sample pool beside them, and an
      # oracle sidecar beside every specimen the corpus can say something about.
      #
      # The corpus repo is the package, so lab consumes what that repo asserts about
      # itself — the git tier filtered against `library.json`, with the standing check
      # that nothing oversized escaped it — rather than whatever a checkout contained.
      #
      # `NORD_CORPUS_DIR` points at this whole store path, not at one model — the sweeps
      # walk every model and join their own subdirectory when they are model-specific.
      nord-corpus = final.callPackage nord-corpus-tree { };

      # The same corpus with the R2 tier projected in — the vendor sample pool, the
      # multi-hundred-MB bundle archives and their untrimmed captures, 7.0GB of it. Not
      # a check: the objects live in a private bucket, so building this needs either R2
      # credentials or a pre-seeded store (`corpus nix-add`), and `nix flake check` must
      # stay runnable without both.
      nord-corpus-full = final.callPackage nord-corpus-tree { full = true; };

      nord-corpus-rev = "43cfa477ac74a6e4f247ae97607b51f581b96aaf";

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
  )
]
