#![cfg(feature = "corpus")]
//! Bit-flip coverage of the `.ne5p` decode: which body bits anything reads.
//!
//! Flip one bit of a specimen's body, restamp the body CRC, parse, and diff the decode
//! field by field. A flip that changes some field, or that the parser refuses outright,
//! is a bit the decode reads. A flip that changes nothing is a bit no field claims — and
//! if the corpus is seen to move that bit on real instruments, it is a hole in the
//! format's coverage rather than a constant.
//!
//! ⚠️ The diff is over [`common::rows`] — typed field values. Rendered output is not a
//! substitute: a display that prints a whole word as one blob, or that declines to show
//! a panel the current model does not select, reports a moved bit as no change.
//!
//! The three counts are ratchets, not reports: coverage is allowed to improve and a
//! regression fails here. Loosening one is a decision to be made deliberately, in the
//! same change that explains it.
//!
//! ```sh
//! NORD_CORPUS_DIR=/path/to/nord-corpus \
//!   cargo test -p nord-format --features corpus --test mutation -- --nocapture
//! ```

mod common;

use common::{all_programs, ne5_dir, read_program, rows};
use nord_format::common::container::Container;
use nord_format::electro5::program::BODY_LEN;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::read;
use std::io::Cursor;

/// First byte of the panel body in a type-1 file.
///
/// ⚠️ Bits are counted from the start of the **body**, which is where the container
/// hands it over; this offset is only how [`spell`] writes a bit down, in the absolute
/// type-1 form the panel modules and the format notes use. A type-0 specimen holds the
/// same 121-byte body 20 bytes earlier.
const BODY: usize = 0x2c;
/// Bits of body a program has. The measurement's denominator.
const BODY_BITS: usize = BODY_LEN * 8;

/// Base specimens to flip, one per shape of program the decode branches on.
///
/// One base cannot answer the question on its own: a field only reachable for one organ
/// model, or only refused for some values, looks unread from a specimen that does not
/// select it. The measurement is the union.
const BASES: &[&str] = &[
    "programs/organ/1000000876543210.ne5p",
    "programs/organ/1100_040000000.ne5p",
    "programs/organ/1400_rqpondcba.ne5p",
    "programs/piano/0000_02_01.ne5p",
    "programs/sample/100_42_000_d000.ne5p",
    "usb/backup/full_backup/contents/Program/Bank 1/Amped Vox.ne5p",
];

/// The accepted coverage floors. **Ratchets**: `>=` for what is read, `<=` for what is
/// not, so a decode that reads more only ever tightens them.
const MIN_VARYING: usize = 556;
const MIN_READ: usize = 536;
const MAX_BLIND: usize = 68;

/// One specimen's container, in whichever generation it was stored in.
fn container(path: &std::path::Path) -> Container {
    let bytes = read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Container::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// `0x51[3]` — where a body bit sits, as a reader of the format notes would write it.
fn spell(bit: usize) -> String {
    format!("{:#04x}[{}]", BODY + bit / 8, bit % 8)
}

/// Flip one body bit and re-emit the file. The container restamps its own checksum, so
/// the parse tests the field decode rather than the checksum.
fn flip(base: &Container, bit: usize) -> Vec<u8> {
    let mut file = base.clone();
    file.body[bit / 8] ^= 1 << (bit % 8);
    file.to_bytes().unwrap()
}

/// The fields whose decode a flip moved, or `None` if the parser refused the file.
///
/// A refusal counts as read: the bits reached a validation, which is the strongest form
/// of being looked at.
fn moved(baseline: &[String], mutated: &[u8]) -> Option<Vec<String>> {
    let program = match nord_format::from_stream(&mut Cursor::new(mutated)) {
        Ok(nord_format::Entity::Program(nord_format::Program::Electro5(p))) => p,
        Ok(other) => panic!("a flipped program decoded as {other:?}"),
        Err(_) => return None,
    };

    Some(
        rows(&program)
            .iter()
            .zip(baseline)
            .filter(|(row, was)| &&render(row) != was)
            .map(|(row, _)| row.key.clone())
            .collect(),
    )
}

/// One row as the diff sees it: placement and both views of the value, since a field can
/// move its bits without moving its rendering and either is a change.
fn render(row: &common::Row) -> String {
    format!("{} {} {}", row.placement, row.raw_str(), row.value)
}

/// Body bits the corpus is seen to move, i.e. the ones a specimen could disagree about.
///
/// A bit no specimen ever moves is not evidence of a hole — it may be a constant the
/// instrument always writes — so the blind-spot count is scoped to these.
fn varying_bits() -> BTreeSet<usize> {
    let programs = all_programs(&ne5_dir());
    let mut zero = vec![false; BODY_BITS];
    let mut one = vec![false; BODY_BITS];

    for path in &programs {
        let body = container(path).body;
        assert_eq!(body.len(), BODY_LEN, "{} is not a program", path.display());
        for bit in 0..BODY_BITS {
            let set = body[bit / 8] >> (bit % 8) & 1 == 1;
            zero[bit] |= !set;
            one[bit] |= set;
        }
    }

    (0..BODY_BITS).filter(|&b| zero[b] && one[b]).collect()
}

/// Every body bit, measured: how many the corpus moves, how many the decode reads, and
/// how many it moves that nothing reads.
#[test]
fn every_body_bit_is_accounted_for() {
    assert_eq!(BODY_BITS, 968, "the body changed size");

    let root = ne5_dir();
    let varying = varying_bits();

    // Bit -> the fields some base saw move, so the number's membership is inspectable
    // and not just its size.
    let mut readers: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    let mut refused: BTreeSet<usize> = BTreeSet::new();

    for base in BASES {
        let path = root.join(base);
        assert!(path.is_file(), "base specimen {base} is missing");
        let base = container(&path);
        let baseline: Vec<String> = rows(&read_program(&path)).iter().map(render).collect();

        for bit in 0..BODY_BITS {
            match moved(&baseline, &flip(&base, bit)) {
                None => {
                    refused.insert(bit);
                }
                Some(fields) => readers.entry(bit).or_default().extend(fields),
            }
        }
    }

    let read: BTreeSet<usize> = readers
        .iter()
        .filter(|(_, fields)| !fields.is_empty())
        .map(|(&bit, _)| bit)
        .chain(refused.iter().copied())
        .collect();
    let blind: Vec<usize> = varying
        .iter()
        .copied()
        .filter(|b| !read.contains(b))
        .collect();

    println!(
        "{BODY_BITS} body bits: {} vary in the corpus, {} are read by some decode \
         ({} of them by refusing the file), {} vary and are read by nothing",
        varying.len(),
        read.len(),
        refused.len(),
        blind.len(),
    );
    println!(
        "read by nothing: {:?}",
        blind.iter().map(|&b| spell(b)).collect::<Vec<_>>()
    );

    assert!(
        varying.len() >= MIN_VARYING,
        "only {} body bits vary across the corpus, down from {MIN_VARYING} — specimens \
         went missing, or they are all one program",
        varying.len(),
    );
    assert!(
        read.len() >= MIN_READ,
        "only {} body bits are read, down from {MIN_READ}: the decode lost coverage",
        read.len(),
    );
    assert!(
        blind.len() <= MAX_BLIND,
        "{} varying body bits are read by nothing, up from {MAX_BLIND}: {:?}",
        blind.len(),
        blind.iter().map(|&b| spell(b)).collect::<Vec<_>>(),
    );
}
