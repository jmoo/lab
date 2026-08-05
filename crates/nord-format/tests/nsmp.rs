#![cfg(feature = "corpus")]
//! Sample-instrument (`.nsmp`) tests, read field by field.
//!
//! `.nsmp` is a cross-model entity (`src/common/sample/`), not an Electro 5 format — but
//! the specimens live under `<corpus>/ne5/samples/` because they were captured off the
//! Electro 5, the only model the corpus currently covers. A model-specific home for the
//! files is not a claim that the format is model-specific.
//!
//! Gated behind the `corpus` cargo feature: these need the specimen corpus, which lives
//! in the private `jmoo/nord-corpus` repo. Without the feature the whole file compiles
//! out, so the default `cargo test` runs the open minimal suite and `tests/fixtures.rs`.
//! The Nix `nord-format-corpus` check enables the feature and sets `NORD_CORPUS_DIR`.
//!
//! ```sh
//! NORD_CORPUS_DIR=/path/to/nord-corpus \
//!   cargo test --workspace --features nord-usb/corpus,nord-format/corpus
//! ```

mod common;

use common::{ne5_dir, read_sample};
use std::fs::read;
use std::path::PathBuf;

fn samples_dir() -> PathBuf {
    ne5_dir().join("samples")
}

/// The named multi-zone specimens, decoded field by field. The filename is the oracle.
#[test]
fn test_nsmp_named_specimens() {
    let dir = samples_dir();
    for (stem, name, roots, tops) in [
        ("D1-one-zone", "TEST", vec![60u8], vec![84u8]),
        ("E-name-14char", "D1-one-zone-C4", vec![60], vec![84]),
        ("D2-rootkey-C3", "TEST", vec![48], vec![72]),
        ("D3-2zones", "D3-2zones", vec![60, 48], vec![84, 53]),
        ("D4-3zones", "D4-3zones", vec![72, 60, 48], vec![96, 65, 53]),
        ("D8-2zones-hi", "D8-2zones-hi", vec![72, 60], vec![96, 65]),
    ] {
        let sample = read_sample(&dir.join(format!("{stem}.nsmp")));
        assert_eq!(sample.name().unwrap(), name, "{stem}: name");
        assert_eq!(sample.header.version, 200, "{stem}: content version");
        assert_eq!(
            sample
                .strokes()
                .unwrap()
                .iter()
                .map(|s| s.root_key)
                .collect::<Vec<_>>(),
            roots,
            "{stem}: root keys"
        );
        assert_eq!(
            sample
                .zones()
                .unwrap()
                .iter()
                .map(|z| z.top_note)
                .collect::<Vec<_>>(),
            tops,
            "{stem}: zone top notes"
        );
    }
}

/// Rename and remap reproduce a specimen the editor itself wrote.
///
/// `D7-upperkey` is `D4-3zones` with the middle zone's upper key moved and a new name.
/// Making the same two edits must give back the same bytes — which also pins that a remap
/// leaves the encoded audio alone, and that the checksum is recomputed correctly.
#[test]
fn test_nsmp_edits_reproduce_the_editors_own_output() {
    let dir = samples_dir();
    let mut sample = read_sample(&dir.join("D4-3zones.nsmp"));

    sample.set_name("D7-upperkey").unwrap();
    sample.set_zone_top_note(1, 60).unwrap();

    assert_eq!(
        sample.to_bytes().unwrap(),
        read(dir.join("D7-upperkey.nsmp")).unwrap(),
        "rename + remap did not reproduce the editor's own file"
    );
}

/// Retuning a zone moves the root key and nothing else but the checksum.
#[test]
fn test_nsmp_retune_touches_one_byte() {
    let dir = samples_dir();
    let before = read(dir.join("D1-one-zone.nsmp")).unwrap();
    let mut sample = read_sample(&dir.join("D1-one-zone.nsmp"));

    sample.set_root_key(0, 48).unwrap();
    let after = sample.to_bytes().unwrap();

    let differing: Vec<usize> = (0..before.len())
        .filter(|&i| before[i] != after[i])
        .collect();
    // The checksum at 0x18..0x1c, plus the one root-key byte.
    assert_eq!(differing.len(), 5, "changed bytes: {differing:?}");
    assert!(differing[..4].iter().eq([0x18, 0x19, 0x1a, 0x1b].iter()));
    assert_eq!(sample.strokes().unwrap()[0].root_key, 48);
}

/// A name longer than the writer is willing to emit is refused, not truncated.
#[test]
fn test_nsmp_overlong_name_is_refused() {
    let mut sample = read_sample(&samples_dir().join("D1-one-zone.nsmp"));
    assert!(sample.set_name("a name that is far too long").is_err());
    assert_eq!(
        sample.name().unwrap(),
        "TEST",
        "a refused rename still changed it"
    );
}

/// A corrupted body is refused on read rather than decoded.
#[test]
fn test_nsmp_bad_checksum_is_refused() {
    let mut bytes = read(samples_dir().join("D1-one-zone.nsmp")).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    assert!(nord_format::from_stream(&mut std::io::Cursor::new(&bytes)).is_err());
}
