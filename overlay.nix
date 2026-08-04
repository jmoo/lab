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
      # ⚠️ The tree carries `library/library.json`, which is the R2 address of every
      # R2-tier object. This repo is public, so nothing may hand a consumer the raw
      # tree: what lab exposes is `mkCorpus`'s output, which is the filtered git tier
      # plus, only when asked, the objects that index names.
      nord-corpus-tree = builtins.fetchGit {
        # ⚠️ The pinned rev lives on `size-tiering`, not the default branch, and
        # `fetchGit` only fetches the refs it is told about — without this it reports
        # the rev as not found. Drop it when the branch merges.
        ref = "size-tiering";
        rev = final.nord-corpus-rev;
        url = "git+ssh://git@github.com/jmoo/nord-corpus.git";
      };

      # The corpus's own assembly, so lab consumes what the corpus repo asserts about
      # itself (git tier filtered against the library index, no oversized file outside
      # it) rather than whatever a checkout happened to contain.
      mkCorpus = import "${nord-corpus-tree}/nix/corpus.nix" {
        pkgs = final;
        src = nord-corpus-tree;
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

      # The specimen corpus nord-format and nord-usb are tested against: seven model
      # directories plus the shared sample library, at the pinned revision.
      #
      # `NORD_CORPUS_DIR` points at this whole store path, not at one model — the sweeps
      # walk every model and join their own subdirectory when they are model-specific.
      nord-corpus = mkCorpus { };

      # The same corpus with the whole R2 tier spliced in — every object
      # `library/library.json` indexes: the multi-hundred-MB bundle archives and their
      # untrimmed captures at their capture paths, and the vendor sample pool at
      # `library/pool/<filename>`, alongside the in-git specimens.
      #
      # Not a check: it needs either R2 credentials or a pre-seeded store
      # (`corpus nix-add`), and `nix flake check` must stay runnable without either.
      # See `docs/nord-corpus.md`.
      nord-corpus-full = mkCorpus { library = true; };

      # The corpus revision this workspace is pinned to, and the only place it is written.
      #
      # ⚠️ The crates' corpus suites parse this binding out of this file at test time to
      # refuse a checkout at another revision — they match `nord-corpus-rev = "<40 hex>";`
      # and fail loudly on anything but exactly one match. Keep it a literal one-line
      # string, and keep it the only such binding here.
      nord-corpus-rev = "dc0604f576a0646e0da9640a72a826845b5a6947";

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
