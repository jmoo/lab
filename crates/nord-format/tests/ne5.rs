#![cfg(feature = "corpus")]
//! Corpus-backed tests whose subject is not one specimen.
//!
//! The per-specimen sweeps live in `corpus_cases.rs`, which reports each specimen as its
//! own case. What stays here is what a per-specimen harness cannot express: a named
//! specimen read field by field, and the assertions that only hold across the corpus as a
//! whole — the dependency-id bijection over every program in the backup, and the set of
//! live slots the corpus covers.
//!
//! Gated behind the `corpus` cargo feature: these need the specimen corpus, which lives
//! in the private `jmoo/nord-corpus` repo (it grows to hold proprietary piano/sample
//! data). Without the feature the whole file compiles out, so the default `cargo test`
//! runs the open minimal suite and `tests/fixtures.rs`. The Nix `nord-format-corpus`
//! check enables the feature and sets `NORD_CORPUS_DIR`.
//!
//! ```sh
//! NORD_CORPUS_DIR=/path/to/nord-corpus \
//!   cargo test --workspace --features nord-usb/replay,nord-format/corpus
//! ```

mod common;

use common::{files_with, ne5_dir, read_program, read_sample, read_settings};
use nord_format::common::bank::Item;
use nord_format::electro5::{Instrument, PianoCategory, SplitPoint};
use nord_format::{electro5, Entity};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::fs::read;
use std::path::PathBuf;

fn read_song(path: &std::path::Path) -> electro5::Song {
    match nord_format::from_path(path).unwrap() {
        Entity::Song(nord_format::Song::Electro5(song)) => song,
        other => panic!("{} is not an Electro 5 song: {other:?}", path.display()),
    }
}

/// The standalone song, read field by field. Its four program references and its own slot
/// are the whole body.
#[test]
fn test_ne5_read_song() {
    let song = read_song(&ne5_dir().join("song.ne5t"));

    assert_eq!(song.location(), (0, 2));
    assert_eq!(song.get(0), (5, 9));
    assert_eq!(song.get(1), (0, 1));
    assert_eq!(song.get(2), (0, 2));
    assert_eq!(song.get(3), (5, 8));
}

/// One program read field by field, as a readable statement of what a specimen holds.
/// The corpus-wide version of this is `decode_snapshot.rs`.
#[test]
fn test_ne5_read_program() {
    let program =
        read_program(&ne5_dir().join("programs/center_panel/o00_1_p000_0_1_0_50_50.ne5p"));
    let center = &program.schema.center_panel;

    assert_eq!(program.location(), (7, 3));
    assert_eq!(center.lower_part, Instrument::Organ);
    assert_eq!(center.upper_part, Instrument::Piano);
    assert_eq!(center.lower_octave_shift, 1);
    assert_eq!(center.upper_octave_shift, 0);
    assert!(!center.lower_sustain);
    assert!(!center.upper_sustain);
    assert!(!center.lower_control);
    assert!(!center.upper_control);
    assert!(!center.split);
    assert_eq!(center.split_point, SplitPoint::F4);
    assert_eq!(center.transpose, 1);
    assert!(!center.transpose_enabled);
}

/// The set list song is the one selection field the sweep never moves, so the two
/// specimens that do move it are what pin it.
#[test]
fn test_ne5_settings_set_list_song_decodes() {
    let root = ne5_dir();
    // Captured before the sweep, in set list mode.
    let early = read_settings(&root.join("settings.ne5s")).schema.selection;
    assert!(early.set_list_mode);
    assert_eq!(early.song.inner(), (0, 1));
    assert_eq!(early.program.inner(), (4, 21));

    // The full backup. Its archive holds exactly two set lists, 1 and 3, and this is the
    // only specimen pointing outside the first.
    let backup = read_settings(
        &root.join("usb/backup/full_backup/contents/Settings/Settings/Settings.ne5s"),
    )
    .schema
    .selection;
    assert!(!backup.set_list_mode);
    assert_eq!(backup.song.inner(), (2, 3));
}

// ---------------------------------------------------------------------------
// Piano / sample dependency ids, across the whole backup
// ---------------------------------------------------------------------------
//
// Both panels carry a 32-bit reference to the piano (`.npno`) or sample (`.nsmp`) the
// program needs. Everything else in those panels is a *slot coordinate* — the piano
// category + model dials, the Samp Lib number — which moves when the instrument's library
// changes. The id is the stable key: it is what resolves the song -> program -> piano
// chain, and what Nord Sound Manager checks before offering a Restore.
//
// The value of each id is pinned per specimen in `corpus_cases.rs`, against the numbers
// the USB captures put on the wire. What this file adds is the two statements that need
// every program at once: across the backup each id must occupy exactly one (category,
// model) slot and vice versa — a too-narrow id over-splits a slot into several ids, a
// too-wide one merges distinct pianos under one id — and the model slots the programs
// reference must be exactly the ones the backup shipped.

/// Piano `category` as stored, and the backup directory it corresponds to. The dial order
/// on disk starts at Grand, so the two are not in step.
const PIANO_CATEGORIES: [(PianoCategory, &str); 6] = [
    (PianoCategory::Grand, "Grand"),
    (PianoCategory::Upright, "Upright"),
    (PianoCategory::EPiano1, "EPiano1"),
    (PianoCategory::EPiano2, "EPiano2"),
    (PianoCategory::Clavinet, "Clavinet"),
    (PianoCategory::Harpsichord, "Harps"),
];

#[test]
fn test_ne5_backup_dependency_ids() {
    let backup = ne5_dir().join("usb/backup/full_backup");

    // The backup's member list: what the instrument actually shipped. The blobs
    // themselves are private-tier and absent here, but the listing tells us how many
    // pianos each category held.
    let members = fs::read_to_string(backup.join("backup.members.tsv")).unwrap();
    let mut shipped: BTreeMap<&str, usize> = BTreeMap::new();
    let mut samples = 0usize;

    for line in members.lines().skip(1) {
        let Some(name) = line.split('\t').next() else {
            continue;
        };

        if name.ends_with(".nsmp") {
            samples += 1;
        } else if name.ends_with(".npno") {
            let mut parts = name.split('/');
            let (Some("Piano"), Some(category)) = (parts.next(), parts.next()) else {
                panic!("unexpected piano member path: {name}")
            };
            *shipped.entry(category).or_default() += 1;
        }
    }
    assert!(
        !shipped.is_empty() && samples > 0,
        "member list looks empty"
    );

    let mut slot_of: BTreeMap<u32, (PianoCategory, u8)> = BTreeMap::new();
    let mut id_of: BTreeMap<(PianoCategory, u8), u32> = BTreeMap::new();
    let mut sample_ids: BTreeSet<u32> = BTreeSet::new();
    let mut programs = 0usize;

    for path in files_with(&backup.join("contents/Program"), "ne5p") {
        let name = path.display().to_string();
        let schema = read_program(&path).schema;
        let (piano, sample) = (&schema.piano_panel, &schema.sample_panel);

        if piano.id != 0 {
            let slot = (piano.category, piano.piano_model.as_u8());

            assert_eq!(
                *slot_of.entry(piano.id).or_insert(slot),
                slot,
                "piano id {:#010x} spans more than one (category, model) slot, at {name}",
                piano.id,
            );
            assert_eq!(
                *id_of.entry(slot).or_insert(piano.id),
                piano.id,
                "slot {slot:?} names more than one piano id, at {name}",
            );
        }

        if sample.id != 0 {
            sample_ids.insert(sample.id);
        }

        programs += 1;
    }

    assert!(
        programs > 0,
        "no backup programs found — is the corpus present?"
    );

    // Per category: the model slots the programs reference must be exactly `0..n`, and
    // `n` must be what the backup shipped — with one exception. The programs reference a
    // seventh Upright that the instrument no longer holds, and that single dangling
    // reference is the whole reason this field matters: it is what Nord Sound Manager
    // sees as a missing dependency and gates "Restore" on. If this trips, check whether
    // the corpus gained or lost a piano before assuming the decode moved.
    for (category, directory) in PIANO_CATEGORIES {
        let models: BTreeSet<u8> = id_of
            .keys()
            .filter(|(c, _)| *c == category)
            .map(|(_, model)| *model)
            .collect();
        let expected = shipped[directory] + usize::from(directory == "Upright");

        assert_eq!(
            models.len(),
            expected,
            "{directory}: programs reference {} model slots, expected {expected}",
            models.len(),
        );
        assert!(
            models.iter().copied().eq(0..expected as u8),
            "{directory}: model slots are not contiguous from 0: {models:?}",
        );
    }

    // Samples have no slot coordinate to cross-check against — `number` is a volatile
    // Samp Lib position that the corpus reuses across ids — so bound the id count by the
    // shipped library instead.
    assert!(
        sample_ids.len() <= samples,
        "{} distinct sample ids referenced but only {samples} `.nsmp` members shipped",
        sample_ids.len(),
    );
}

/// The three live slots are `0:0`, `0:1` and `0:2` on the wire, and nothing else.
#[test]
fn test_ne5_live_occupies_three_slots_of_one_bank() {
    let mut seen: BTreeSet<(u16, u16)> = BTreeSet::new();

    for path in files_with(&ne5_dir(), "ne5l") {
        let name = path.display().to_string();
        let Entity::Live(nord_format::Live::Electro5(live)) =
            nord_format::from_path(&path).unwrap()
        else {
            panic!("expected an electro5 live slot in {name}")
        };
        let at = live.location();
        assert_eq!(at.x(), 0, "live slot outside bank 0 in {name}");
        seen.insert(at.inner());
    }

    assert_eq!(
        seen,
        BTreeSet::from([(0, 0), (0, 1), (0, 2)]),
        "the corpus no longer covers all three live slots",
    );
}

// ---------------------------------------------------------------------------
// Sample instruments (`.nsmp`)
// ---------------------------------------------------------------------------

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
        sample.to_bytes(),
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
    let after = sample.to_bytes();

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
