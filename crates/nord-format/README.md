# nord-format

Parse and write **Clavia / Nord** keyboard binary file formats from Rust.

This is the pure format-logic crate of the Nord toolkit: the `CBIN` container in
both of its header generations, their two checksums, and per-model entity
layouts. It depends
only on [`binrw`] (plus `zip` behind the `bundle` feature for backup bundles) and
does no USB, OS, or I/O beyond `Read`/`Seek`/`Write` — so it's trivially testable
against a specimen corpus and reusable by higher layers (a device/USB crate, a
CLI) without dragging in a transport stack.

## What it handles

| Format | Parse | Write (byte-exact) | Semantic decode |
|---|:--:|:--:|---|
| `ne5p` program | ✅ | ✅ | Center / piano / sample / FX / EQ panels ✅. **Organ**: drawbars, preset, vibrato/chorus (all models) and B3 percussion ✅ — B3-bass & Farfisa drawbar *display* transforms still pending. |
| `ne5t` song / set | ✅ | ✅ | ✅ (four program slots) |
| `ne5l` live slot | ✅ | ✅ | Same as `ne5p` — the live buffer is the program body under another tag (confirmed on hardware), so it shares the program schema in the three live slots. |
| `ne5s` settings | ✅ | ✅ | System / MIDI / Sound menus plus the power-up selection ✅ — 32 of the 34 catalogued settings are pinned by a change-one-setting hardware sweep; memory protect and local control move no bit of the body and stay undecoded. |
| `npno` piano / `nsmp` sample | ✅ (header) | ⬜ header only | Only the header is parsed and re-emitted, so a full library file does **not** round-trip yet — `nord verify` on one reports the truncation. |
| backup bundle (ZIP) | ✅ | — | Partial; behind the `bundle` feature. |

Everything that parses **round-trips byte-for-byte**, verified against a
change-one-knob specimen corpus.

### Both CBIN generations

A file's header type sits at `0x04` and decides the container, not the body:

| | type 1 | type 0 |
|---|---|---|
| header | 44 bytes | 24 bytes |
| checksum | CRC-32 (ISO-HDLC) at `0x18`, over the body | CRC-16 (IBM-3740) in the last two bytes, over everything before them |
| body starts | `0x2c` | `0x18` |

so the same content is 18 bytes shorter as type 0. `common::container` reads and
writes both, and every reader re-emits a file in the generation it arrived in.
Type 0 is the commoner of the two across the corpus: the Electro 5 factory banks,
the Electro 3 / 3 HP / 4 / 4D, both C2s, the Wave, the Stage Classic / EX / 2 /
2 EX, the Nord Piano 1–3, the Lead 4 and A1, and the whole `.nsmp` sample
generation. Type 1 is what the Electro 5 writes in the field, and what the
Electro 6 and 7, the Stage 3 and 4, the Nord Piano 4 and 5, the Grand, the Organ
3 and the Wave 2 ship.

⚠️ A type-0 file's crc16 covers its **header** as well as its body, so patching a
tag in place invalidates it where the type-1 crc32 would not notice.

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
  (`tests/corpus_cases.rs` and friends); see below.

## Tests

Unit tests live inline (`#[cfg(test)] mod tests`) and run on a plain `cargo test`,
alongside `tests/fixtures.rs` — a set of small **synthetic** specimens in
`tests/fixtures/`, emitted by this crate's own writers, which give a fresh clone a
parse / round-trip / checksum suite with no corpus at all. Their filenames are the
oracle, and `cargo test -p nord-format --test fixtures -- --ignored bless` rewrites
the bytes from the generator.

The **corpus integration suite** is gated behind the `corpus` feature because it needs
the specimen corpus, which lives in a separate private repo (`jmoo/nord-corpus`).
`tests/corpus_cases.rs` is a `libtest-mimic` harness reporting one case per specimen
(`<check>/<path under the corpus root>`), so a failure names the file, all failures
surface in one run, and a `.skip.` specimen shows up as an ignored case. `tests/ne5.rs`
holds what a per-specimen harness cannot express — a named specimen read field by
field, and the assertions that span the whole corpus.

`tests/containers.rs` is the sweep across **every** instrument the corpus covers —
32 model directories and the shared sample library, ten thousand specimens in the
git tier and 12,646 with the whole R2 tier spliced in. Each file gets two labels.

Its **class** is what the first four bytes say it is, and is a property of the
material: `cbin-type0` / `cbin-type1`, or `midi`, `sysex`, `zip`, `cn3`, `smac`
for the corpus's non-CBIN material — six model directories hold no CBIN at all —
or `unknown`. Its **outcome** is how far this build gets:

| | |
|---|---|
| `decoded` | a schema accounts for the body and it re-emits byte for byte |
| `container` | the container reads and checksums, the body is one unknown region |
| `unsupported` | the magic is recognised and this build does not read the format |
| `refused` | a CBIN file whose container does not read — **the regression signal** |
| `unidentified` | the magic names nothing |

Class counts and outcomes are floored per model and per class, and the floors
ratchet up only. An unknown format is a classification, not a failure, and
`unsupported` is a permanent answer rather than a TODO: the CBIN container and
its two checksums are the whole of what this crate knows, and none of it applies
to a MIDI dump. What fails is a file sliding *down* a class — or an `unknown`
count going *up*, which is the one number here that is capped rather than
floored.

### Differential and factory specimens

The corpus holds two kinds of file, and its rule is machine-checkable: **a path
with a `factory/` component is vendor material and carries no filename oracle**.
Everything else in a model directory is differential — a change-one-knob patch
whose filename encodes its settings.

So the oracle-driven checks filter on `is_factory` (`tests/common/mod.rs`) and a
`coverage/no_factory_in_the_oracle_sweep` trial asserts none leaked in, while
round-trip and container sweeps take the whole tree. Nothing here needs a
hand-maintained exclusion list, and a new model directory cannot break one.

`NORD_CORPUS_DIR` names the **corpus root** — the directory holding `ne5/`,
`ne6/`, … and `library/` — not one model. A suite that is about one instrument
joins its own directory (`ne5_dir()`).

```sh
cargo test -p nord-format                       # minimal suite (inline unit tests)

# Full corpus sweep — point NORD_CORPUS_DIR at a nord-corpus checkout root:
NORD_CORPUS_DIR=/path/to/nord-corpus \
  cargo test -p nord-format --features corpus

# Everything both crates have, which is what a change should be run against:
NORD_CORPUS_DIR=/path/to/nord-corpus \
  cargo test --workspace --features nord-usb/corpus,nord-format/corpus

# With nix
nix build .#checks.<system>.nord-format-corpus
```

The Nix check runs against the corpus revision pinned in the flake's `overlay.nix`;
a local run is only as pinned as the checkout `NORD_CORPUS_DIR` names.
`tests/corpus_guard.rs` runs under every feature set and fails when `NORD_CORPUS_DIR`
is set without `--features corpus`, since that run verifies none of the decode.

## Where this fits

This project started in 2023 as my personal 'learn rust' project. The goal was to be able to read/write clavia files for my electro 5. After I got most of the ne5 formats RE'd, I got a new job that was pretty demanding and I didn't have much free time to continue.

The goal now is to finish reverse engineering ne5 files, add support for more models, and to reverse engineer the USB protocol. The end game is to have a portable toolkit that could be the foundation of an open source nord manager alternative with linux support.

## Current State

Messy and incomplete but solid for reading and writing electro 5 files due to lossless round trips and large test corpus.
Nord Electro 5 Program, Song, Live and Settings files are nearly 100% solved for the file versions I've encountered, and the library safely errors when
encountering unexpected files or versions. Bundles, backups, piano, and sample files are still incomplete.

Expect refactoring, API changes, and other misc changes until a stable version is released. I would also like to support
more nord models, but will not work on those until I'm 100% satisfied with the Electro 5 support.

## Prior art

 Shout out to @Chris55. I wish I had discovered his work before I started. It would have made some of the initial reverse engineering easier. Once I finish decoding the ne5, I plan to contribute the ICD back to these projects.

- **[`Chris55/ns3-program-viewer`](https://github.com/Chris55/ns3-program-viewer)**
  — a read-only web viewer for Nord Stage 2 / 2EX / 3 programs.
- **[`Chris55/nord-documentation`](https://github.com/Chris55/nord-documentation)**
  ([rendered](https://chris55.github.io/nord-documentation/)) — community byte-map
  docs for Nord Stage 2/3 and Lead A1, built with the same hex-diff method.

Why maintain a separate project? Primarily because this is a rust learning project for me and I've written enough js in my life to not really want to write it in my free time. Also, the goal of this project are r/w + usb, not just read.

## Disclaimer

Not affiliated with, authorized, or endorsed by Clavia DMI AB. "Nord", "Clavia",
and "Electro" are trademarks of Clavia DMI AB, used here only to identify the
hardware these formats come from. All reverse engineering is of files produced by
hardware the author owns, for interoperability.

[`binrw`]: https://docs.rs/binrw
[`Entity`]: https://docs.rs/nord-format
