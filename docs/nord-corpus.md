# The Nord specimen corpus

The `nord-*` crates are tested against a specimen corpus in a **separate private
repo**, pinned by revision in `overlay.nix` and fetched over SSH only when a
corpus target is built. `NORD_CORPUS_DIR` names the corpus root — the directory
holding `ne5/`, `ne6/`, … and `library/`.

```sh
nix build .#checks.<system>.nord-format-corpus   # needs read access to the corpus repo
nix build .#checks.<system>.nord-usb-corpus
```

`overlay.nix`'s `nord-corpus-rev` is the only place that revision is written. The
crates' corpus suites parse the binding out of that file at test time and refuse
a checkout sitting anywhere else, so a local run and a Nix check cannot quietly
read different specimens. Two cases pass: a corpus git cannot answer for is the
Nix store assembly, and a build with no `overlay.nix` beside it is the Nix
sandbox — in both, the corpus in hand *is* the pinned fetch.

`pkgs.nord-corpus` is the corpus's own `nix/corpus.nix` assembly, not the raw
tree: the git tier filtered against the corpus's `library/library.json`, with the
corpus repo's standing assertion that nothing oversized escaped it. Only that
output is exposed, so no R2 address reaches this public repo.

## The everything-run: both suites against the full corpus

`packages.nord-corpus-full` is the same assembly with the whole R2 tier spliced
in — every object `library/library.json` indexes: the multi-hundred-megabyte
bundle archives and their untrimmed captures at their capture paths, and the
1,018-file vendor sample pool under `library/pool/<real filename>`, alongside the
curated specimens that already live in `library/specimens/`.

`packages.corpus-full` runs **both** crates' full suites against it, and is the
one command:

```sh
cd ~/Repos/jmoo/nord-corpus && nix develop
corpus nix-add            # seed the store, ~3.5GB; `corpus doctor` checks R2 setup

cd ~/Repos/jmoo/lab
nix build .#corpus-full
```

It is deliberately **not** a check: it needs that seeded store (or R2 credentials
in the builder), and `nix flake check` has to stay runnable with neither. Either
leg builds on its own as `.#nord-format-corpus-full` / `.#nord-usb-corpus-full`.

Every indexed object is its own fixed-output derivation whose output *is* the
vendor bytes, and `nix-add` stages each one at exactly the store path that
derivation expects — once the path exists Nix considers the derivation satisfied
and never runs the fetch. The alternative is to give the Nix daemon the `R2_*`
variables (in a multi-user install it does not inherit yours), which each
derivation's own error message spells out. Seeded paths carry no GC root — a
`nix-collect-garbage` can reclaim them, and the next build just re-seeds.

> ⚠️ Never push `nord-corpus-full` or the objects it fetches to a shared binary
> cache. The outputs are vendor firmware and sample libraries under their own
> hashes, and content-addressing is not access control.

To iterate locally instead, build the corpus alone and point the test matrix at
it:

```sh
NORD_CORPUS_DIR=$(nix build .#nord-corpus-full --print-out-paths) \
  cargo test --workspace --features nord-usb/corpus,nord-format/corpus
```

`nord-format`'s container sweep (`tests/containers.rs`) sees the pool
automatically — `library/pool` gets one trial per file plus completeness floors
read against the corpus's own `library/library.json`. Against a plain
`pkgs.nord-corpus` (no pool), the sweep still runs; it just reports one visible
ignored trial, `library_pool/absent`, instead of silently covering fewer files.
