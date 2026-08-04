# The Nord specimen corpus

The `nord-*` crates are tested against a specimen corpus in a **separate private
repo**, pinned by revision in `crates/corpus_rev.txt` and fetched over SSH only
when a corpus target is built. `NORD_CORPUS_DIR` names the corpus root — the
directory holding `ne5/`, `ne6/`, … and `library/`.

It covers 32 model directories across the Electro, Stage, Piano, Grand, Wave,
Lead, C2, Organ and Drum families, plus a sample library shared between them.
Ten thousand specimens are in git; the vendor sample pool and a handful of
oversized originals are not — see the tiers below.

```sh
nix build .#checks.<system>.nord-format-corpus   # needs read access to the corpus repo
nix build .#checks.<system>.nord-usb-corpus
```

## Three ways to run

| | Corpus | What it covers |
|---|---|---|
| `cargo test --workspace` | none | The crates' own unit tests and synthetic fixtures. The corpus suites compile out; the guards fail the run if `NORD_CORPUS_DIR` is set anyway. |
| `nix build .#checks.<system>.nord-{format,usb}-corpus` | `pkgs.nord-corpus` — the git tier | Every committed specimen. Needs read access to the corpus repo and nothing else. |
| `nix build .#corpus-full` | `pkgs.nord-corpus-full` — git tier **plus** the whole R2 tier | The above, and the 2,433-file vendor sample pool and the originals git cannot hold. Needs a seeded store. |

`crates/corpus_rev.txt` is the only place that revision is written. `overlay.nix`
reads it to fetch the corpus, and the crates' corpus suites read the same file at
test time and refuse a checkout sitting anywhere else, so a local run and a Nix
check cannot quietly read different specimens. One case passes: a corpus git
cannot answer for is the Nix store assembly, where the corpus in hand *is* the
pinned fetch by construction.

`pkgs.nord-corpus` is the corpus's own `nix/corpus.nix` assembly, not the raw
tree: the git tier filtered against the corpus's `library/library.json`, with the
corpus repo's standing assertion that nothing oversized escaped it. Only that
output is exposed, so no R2 address reaches this public repo.

## The everything-run: both suites against the full corpus

`packages.nord-corpus-full` is the same assembly with the whole R2 tier spliced
in — every object `library/library.json` indexes: the bundle archives, their
untrimmed captures and the C2's 59MB pipe-organ library back at their own paths,
and the 2,433-file vendor sample pool under `library/pool/<real filename>`,
alongside the curated specimens that already live in `library/specimens/`. It
costs 7.1GB on disk.

`packages.corpus-full` runs **both** crates' full suites against it, and is the
one command:

```sh
cd ~/Repos/jmoo/nord-corpus && nix develop
corpus nix-add            # seed the store, ~7GB; `corpus doctor` checks R2 setup

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

`nord-format`'s container sweep (`tests/containers.rs`) sees the whole R2 tier
automatically — every spliced-in file gets the same per-file trial the git tier
does, and `library/pool` adds completeness floors read against the corpus's own
`library/library.json`. Against a plain `pkgs.nord-corpus` (no pool), the sweep
still runs; it just reports one visible ignored trial, `library_pool/absent`,
instead of silently covering fewer files. The git tier is 10,196 trials and the
full corpus 12,646.
