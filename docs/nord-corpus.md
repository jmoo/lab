# The Nord specimen corpus

The `nord-*` crates are tested against a specimen corpus in a **separate private
repo**, pinned by revision in `overlay.nix` and fetched over SSH only when a
corpus target is built. `NORD_CORPUS_DIR` names the corpus root — the directory
holding `ne5/`, `ne6/`, … and `library/`.

```sh
nix build .#checks.<system>.nord-format-corpus   # needs read access to the corpus repo
nix build .#checks.<system>.nord-usb-corpus
nix build .#checks.<system>.corpus-rev-agrees    # needs nothing
```

`pkgs.nord-corpus` is the corpus's own `nix/corpus.nix` assembly, not the raw
tree: the git tier filtered against the corpus's R2 manifest, with the corpus
repo's standing assertion that nothing oversized escaped it. Only that output is
exposed, so no blob address from the manifest reaches this public repo.

## Building the full corpus: bundles and the sample pool

`packages.nord-corpus-full` is the same assembly with two private-bucket things
spliced in: the manifest's R2-tier blobs (the multi-hundred-megabyte bundle
archives and their untrimmed captures) and the whole 1,018-file vendor sample
pool, under `library/pool/<real filename>`, alongside the 19 curated specimens
that already live in `library/specimens/`. It is deliberately **not** a check:
both live in a private bucket, and `nix flake check` has to run without
credentials for either.

Each item — each blob, each pool file — is its own fixed-output derivation whose
output *is* the vendor bytes, so the simplest way to build any of it needs no
privilege change: fetch it yourself and hand it to the store at exactly the path
the derivation expects.

```sh
cd ~/Repos/jmoo/nord-corpus && nix develop
corpus nix-add <artifact-id>           # once per bundle blob; `corpus doctor` checks R2 setup
corpus nix-add --pool                  # seeds all 1,018 pool files from local disk —
                                        # $NORD_SAMPLE_POOL, or after `corpus r2 pull`

nix build .#nord-corpus-full
```

Once a store path exists Nix considers that derivation satisfied and never runs
the fetch. The alternative is to give the Nix daemon the `R2_*` variables (in a
multi-user install it does not inherit yours), which each derivation's own error
message spells out. Seeded paths carry no GC root — a `nix-collect-garbage` can
reclaim them, and the next build just re-seeds.

> ⚠️ Never push `nord-corpus-full` or its blobs to a shared binary cache. The
> outputs are vendor firmware and sample libraries under their own hashes, and
> content-addressing is not access control.

With the full corpus built, point the whole test matrix at it:

```sh
NORD_CORPUS_DIR=$(nix build .#nord-corpus-full --print-out-paths) \
  cargo test --workspace --features nord-usb/corpus,nord-format/corpus
```

`nord-format`'s container sweep (`tests/containers.rs`) sees the pool
automatically — `library/pool` gets one trial per file plus completeness floors
read against the corpus's own `library/library.json`. Against a plain
`pkgs.nord-corpus` (no pool), the sweep still runs; it just reports one visible
ignored trial, `library_pool/absent`, instead of silently covering fewer files.
