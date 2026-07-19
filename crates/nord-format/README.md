# nord-format

Parse and write **Clavia / Nord** keyboard binary file formats from Rust.

This is the pure format-logic crate of the Nord toolkit: the `CBIN` container,
the CRC-32 (ISO-HDLC) range checksum, and per-model entity layouts. It depends
only on [`binrw`] (plus `zip` behind the `bundle` feature for backup bundles) and
does no USB, OS, or I/O beyond `Read`/`Seek`/`Write`, so it is trivially testable
against a specimen corpus and reusable by higher layers (a device/USB crate, a
CLI) without dragging in a transport stack.

Ported from the `binrw` generation of `jmoo/nord-utils`; that repo remains the
reverse-engineering workbench (specimens, capture dumps, protocol notes).

## Status

- **Electro 5** (`ne5p` program, `ne5t` song/set, `ne5s` settings) — decoded and
  **corpus-verified**: `parse → write` round-trips byte-for-byte. The organ panel
  is carried as a raw passthrough (framing verified; semantic decode is the known
  gap — see the byte-map in `src/electro5/program.rs`).
- `npno` piano / `nsmp` sample — header only.
- Backup **bundles** (ZIP + entities) behind the `bundle` feature.

## Invariant: lossless round-trip

Unknown regions are represented as raw byte blocks, so `parse → write` is always
byte-identical even where semantics are incomplete. Every new decoded field is a
safe, incremental refinement.

## Features

- `bundle` (opt-in) — ZIP-based backup bundles. Off by default so parse-only
  consumers (and the corpus round-trip tests, which never touch bundles) stay
  lean; enable with `--features bundle`.

## Tests & the specimen corpus

The round-trip tests are driven by a change-one-knob specimen corpus whose
filenames encode the settings. By default they read the crate's committed
`tests/corpus` and **skip** any panel whose fixtures are absent, so a fresh
checkout stays green. Point `NORD_CORPUS_DIR` at the full specimen set in the
`nord-utils` workbench to run the exhaustive sweep across every panel:

```sh
NORD_CORPUS_DIR=~/Repos/jmoo/nord-utils/archive/resources/ne5 cargo test -p nord-format
```

[`binrw`]: https://docs.rs/binrw
