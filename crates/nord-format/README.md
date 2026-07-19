# nord-format

Parse and write **Clavia / Nord** keyboard binary file formats from Rust.

This is the pure format-logic crate of the Nord toolkit: the `CBIN` container,
the CRC-32 (ISO-HDLC) range checksum, and per-model entity layouts. It depends
only on [`binrw`] (plus `zip` behind the `bundle` feature for backup bundles) and
does no USB, OS, or I/O beyond `Read`/`Seek`/`Write` — so it's trivially testable
against a specimen corpus and reusable by higher layers (a device/USB crate, a
CLI) without dragging in a transport stack.

## What it handles

| Format | Parse | Write (byte-exact) | Semantic decode |
|---|:--:|:--:|---|
| `ne5p` program | ✅ | ✅ | Center / piano / sample / FX / EQ panels ✅. **Organ**: drawbars, preset, vibrato/chorus (all models) and B3 percussion ✅ — B3-bass & Farfisa drawbar *display* transforms still pending. |
| `ne5t` song / set | ✅ | ✅ | ✅ (four program slots) |
| `ne5s` settings | ✅ | ✅ | Raw body only — round-trips byte-exact; field decode is pending a specimen corpus (the System/MIDI/Sound catalog is documented on `electro5::settings::Settings`). |
| `npno` piano / `nsmp` sample | ✅ (header) | ✅ | Header / reference only. |
| backup bundle (ZIP) | ✅ | — | Partial; behind the `bundle` feature. |

Everything that parses **round-trips byte-for-byte**, verified against a
change-one-knob specimen corpus.

## Usage

```rust
use nord_format::{from_path, Entity, Program};
use nord_format::common::bank::Item; // for `.location()`
use nord_format::electro5::OrganModel;

let entity = from_path("patch.ne5p")?;

if let Entity::Program(Program::Electro5(p)) = entity {
    println!("location: {:?}", p.location());
    println!("lower/upper: {:?} / {:?}", p.lower_part(), p.upper_part());

    // Organ state is decoded per model + selected preset:
    let preset = p.organ().preset(OrganModel::B3);
    println!("B3 drawbars: {:?}", p.organ().drawbars(OrganModel::B3, preset));
    println!("B3 vibrato:  {:?}", p.organ().vib_type(OrganModel::B3));
}
```

`from_path` / `from_stream` sniff the container and return an [`Entity`]; the
`electro5` module holds the concrete `Program`/`Song`/`Settings` layouts and the
`OrganModel` / `VibChorus` / `PercSpeed` decode types.

## Lossless round-trip is the core invariant

Unknown regions are kept as **raw byte blocks** and decoded values are exposed as
read-only views over them, so `parse → write` is byte-identical even where the
semantics are incomplete. Every newly decoded field is a safe, incremental
refinement — never a risk to the write path.

## Features

- **`bundle`** — ZIP-based backup bundles (pulls in the `zip` stack). Off by
  default so parse-only consumers stay lean; enable with `--features bundle`.
- **`corpus`** — *test-only*. Gates the corpus-backed integration tests
  (`tests/ne5.rs`); see below.

## Tests

Unit tests live inline (`#[cfg(test)] mod tests`) and run on a plain
`cargo test`. The **corpus integration suite** (`tests/ne5.rs`) is gated behind
the `corpus` feature because it needs the specimen corpus, which lives in a
separate private repo (`jmoo/nord-corpus`)

```sh
cargo test -p nord-format                       # minimal suite (inline unit tests)

# Full corpus sweep — point NORD_CORPUS_DIR at a nord-corpus/ne5 checkout:
NORD_CORPUS_DIR=/path/to/nord-corpus/ne5 \
  cargo test -p nord-format --features corpus

# With nix
nix build .#checks.<system>.nord-format-corpus
```


## Where this fits

Part of the Nord toolkit whose north star is a Rust library for interacting with
any Nord model — full file read/write plus USB — feature-complete with the
closed-source Nord Sound Manager. This crate is the format layer; a device/USB
crate and a thin CLI ([`nord-cli`](../nord-cli)) sit on top. Reverse-engineering
notes and the byte maps still being worked out live with the specimen corpus.

## Prior art

Building on the Nord reverse-engineering community rather than from scratch:

- **[`Chris55/ns3-program-viewer`](https://github.com/Chris55/ns3-program-viewer)**
  — a read-only web viewer for Nord Stage 2 / 2EX / 3 programs.
- **[`Chris55/nord-documentation`](https://github.com/Chris55/nord-documentation)**
  ([rendered](https://chris55.github.io/nord-documentation/)) — community byte-map
  docs for Nord Stage 2/3 and Lead A1, built with the same hex-diff method.

Our differentiation is Rust, **writing** (not just reading), and eventually USB.
The intent is two-way: consume those byte maps to bootstrap Stage support, and
contribute the Electro 5 reverse engineering back.

## Disclaimer

Not affiliated with, authorized, or endorsed by Clavia DMI AB. "Nord", "Clavia",
and "Electro" are trademarks of Clavia DMI AB, used here only to identify the
hardware these formats come from. All reverse engineering is of files produced by
hardware the author owns, for interoperability.

[`binrw`]: https://docs.rs/binrw
[`Entity`]: https://docs.rs/nord-format
