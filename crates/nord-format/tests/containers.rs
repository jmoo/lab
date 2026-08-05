//! Every specimen the corpus ships, classified twice: by what its first bytes say it
//! is, and by how far this build gets on it.
//!
//! The corpus covers thirty-odd instruments and a shared sample library, and this build
//! has body schemas for one of them. That is not a reason to look at only one: **the
//! CBIN container is common to most of them**, and a sweep over every model checks the
//! part that is shared, counts what is not, and turns "we do not read `.ns4p`" from a
//! silent absence into a number that cannot quietly get worse.
//!
//! ⚠️ **Not everything here is CBIN.** Six model directories hold none at all — the
//! Leads ship MIDI SysEx bulk dumps (`.syx`, and the same payload wrapped in `.mid`) and
//! the Drums ship ZIP banks — and the sample pool carries `CNE3` files that are a format
//! of their own. So each file gets a [`Class`] from its magic and an [`Outcome`] from
//! this build, and the two are independent axes:
//!
//! * [`Class`] is a property of the material. It never ratchets.
//! * [`Outcome`] is a property of this build. It ratchets up only — see [`OUTCOMES`].
//!
//! ⚠️ **An unknown format is a classification, not a failure.** A `.ns4p` landing in
//! `container` is the honest answer and the suite stays green; a `.syx` landing in
//! `unsupported` is the permanent one, because the CBIN core does not generalise to
//! alien formats and no amount of work here will move it. What fails is a file sliding
//! *down*, which the ratcheted floors catch. Never lower a floor to make a run green —
//! a floor going down is the finding.
//!
//! `library/pool` — the 2,433-file vendor sample pool `mkCorpus { library = true; }`
//! splices in — gets the same per-file sweep automatically (nothing excludes it from the
//! walk), plus completeness floors against the corpus's own `library.json` in
//! [`pool_coverage`]. A plain corpus has no pool at all; that is not silently fewer
//! trials, it is one visible ignored trial saying so. The full corpus also splices back
//! the R2-tier originals the git tier cannot hold — the C2's 59MB `.npip` pipe library
//! and the two `.ne5*bundle` archives — and those are swept like anything else.
//!
//! ```sh
//! NORD_CORPUS_DIR=/path/to/nord-corpus \
//!   cargo test -p nord-format --features corpus --test containers
//! ```

#[cfg(feature = "corpus")]
mod common;

#[cfg(feature = "corpus")]
mod containers {
    use super::common::{corpus_dir, rel};
    use libtest_mimic::{Arguments, Trial};
    use nord_format::common::container;
    use std::collections::BTreeMap;
    use std::fmt::Write as _;
    use std::fs;
    use std::path::{Path, PathBuf};

    // -----------------------------------------------------------------------
    // The classification
    // -----------------------------------------------------------------------

    /// What a file's first bytes say it is.
    ///
    /// A property of the material, not of this build: teaching the crate a format moves
    /// its [`Outcome`], never its class.
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
    pub enum Class {
        /// `CBIN`, carrying the header type at `0x04`. The container this crate reads.
        Cbin(u32),
        /// `MThd` — a standard MIDI file. The Leads' SysEx dumps also ship this way.
        Midi,
        /// `F0` — a raw MIDI SysEx bulk dump.
        Sysex,
        /// `PK\x03\x04` — a ZIP: the Drums' banks, and the Electro 5's transfer bundles.
        Zip,
        /// `CNE3` — the Electro 2 library format. Unexplained beyond the magic; the
        /// corpus records the name as what the bytes show, not as what the provenance
        /// (an Electro *2* archive) would suggest.
        Cne3,
        /// `SMAC` — a Nord Sample Editor project, the source a `.nsmp` is built from.
        Smac,
        /// The magic names nothing this suite knows. Capped, not floored — see
        /// [`UNKNOWN_CAP`].
        Unknown,
    }

    impl Class {
        fn of(bytes: &[u8]) -> Class {
            match bytes {
                [b'C', b'B', b'I', b'N', ..] if bytes.len() >= container::TYPE_AT + 4 => {
                    Class::Cbin(container::header_type(bytes).expect("magic already matched"))
                }
                [b'M', b'T', b'h', b'd', ..] => Class::Midi,
                [0xf0, ..] => Class::Sysex,
                [b'P', b'K', 0x03, 0x04, ..] => Class::Zip,
                [b'C', b'N', b'E', b'3', ..] => Class::Cne3,
                [b'S', b'M', b'A', b'C', ..] => Class::Smac,
                _ => Class::Unknown,
            }
        }

        fn name(self) -> String {
            match self {
                Class::Cbin(t) => format!("cbin-type{t}"),
                Class::Midi => "midi".to_string(),
                Class::Sysex => "sysex".to_string(),
                Class::Zip => "zip".to_string(),
                Class::Cne3 => "cn3".to_string(),
                Class::Smac => "smac".to_string(),
                Class::Unknown => "unknown".to_string(),
            }
        }
    }

    /// How far this build gets on one file. Ordered worst to best, so a floor reads as
    /// "at least this many files reach at least this class".
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
    pub enum Outcome {
        /// The magic names nothing, so there is nothing to say about the file beyond
        /// its first four bytes.
        Unidentified,
        /// A CBIN file whose container does not read: truncated, a header generation
        /// with no known layout, or a checksum that does not hold. **The regression
        /// signal** — every other outcome is a statement about scope.
        Refused,
        /// The magic is recognised and this build does not read the format. **A
        /// permanent outcome, not a TODO**, for MIDI, SysEx and `CNE3`: the CBIN
        /// container and its two checksums are the whole of what this crate knows, and
        /// none of it applies to an alien format.
        Unsupported,
        /// The file reads and does not come back out byte for byte: a CBIN header whose
        /// checksum verifies over a body this build has no schema for, or a ZIP bundle
        /// walked as the entities inside it, which is not a `binrw` structure and has
        /// nothing to re-emit.
        Container,
        /// A schema accounts for the body and the file re-emits byte for byte.
        Decoded,
    }

    impl Outcome {
        fn as_str(self) -> &'static str {
            match self {
                Outcome::Unidentified => "unidentified",
                Outcome::Refused => "refused",
                Outcome::Unsupported => "unsupported",
                Outcome::Container => "container",
                Outcome::Decoded => "decoded",
            }
        }
    }

    /// Classify one file both ways, with the reason in the reader's words.
    fn classify(path: &Path, bytes: &[u8]) -> (Class, Outcome, String) {
        let class = Class::of(bytes);
        let (outcome, why) = match class {
            Class::Cbin(_) => match container::Container::parse(bytes) {
                Err(e) => (Outcome::Refused, e.to_string()),
                Ok(_) => match nord_format::from_path(path) {
                    Ok(_) => (Outcome::Decoded, String::new()),
                    Err(e) => (Outcome::Container, e.to_string()),
                },
            },
            // The one class whose outcome is not fixed by the material: `bundle` reads a
            // ZIP as a walk over the entities inside it, and a `--workspace` build turns
            // that feature on through nord-cli whatever this crate's own command line
            // said. Measured, not assumed — and [`ZIP_OUTCOME`] is the floor's half of
            // the same read.
            Class::Zip => match nord_format::from_path(path) {
                Ok(_) => (
                    Outcome::Container,
                    "bundle: the members read, the archive does not re-emit".to_string(),
                ),
                Err(e) => (Outcome::Unsupported, e.to_string()),
            },
            Class::Unknown => (
                Outcome::Unidentified,
                format!("magic {:02x?}", &bytes[..bytes.len().min(4)]),
            ),
            other => (
                Outcome::Unsupported,
                format!("{} is not a CBIN container", other.name()),
            ),
        };
        (class, outcome, why)
    }

    /// One file's trial: say what it is and what this build did with it, and hold it to
    /// that outcome's contract.
    ///
    /// Three outcomes carry one. `Decoded` must re-emit byte for byte. `Unsupported` and
    /// `Unidentified` must be *refused* — a crate that returned an entity for a MIDI
    /// file would be worse than one that reads nothing. And a `Container` that did read
    /// must not claim to re-emit, since only a `Decoded` file has been shown to.
    fn check(path: PathBuf) {
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let (class, outcome, why) = classify(&path, &bytes);
        match outcome {
            Outcome::Unidentified | Outcome::Unsupported => {
                println!("{}: {why}", class.name());
                assert!(
                    nord_format::from_path(&path).is_err(),
                    "{} is {} and this crate read it as an entity anyway",
                    path.display(),
                    class.name(),
                );
            }
            Outcome::Refused => println!("refused at the container: {why}"),
            Outcome::Container => {
                println!("container reads, body unknown: {why}");
                if let Ok(entity) = nord_format::from_path(&path) {
                    assert!(
                        nord_format::to_bytes(&entity).is_err(),
                        "{} re-emitted without a schema accounting for its body — that is \
                         a `decoded` file and this sweep called it `container`",
                        path.display(),
                    );
                }
            }
            Outcome::Decoded => {
                let entity = nord_format::from_path(&path).unwrap();
                let again = nord_format::to_bytes(&entity).unwrap_or_else(|e| {
                    panic!("{} decoded but would not re-emit: {e}", path.display())
                });
                assert_eq!(
                    again.as_slice(),
                    bytes.as_slice(),
                    "re-encoding changed {}",
                    path.display(),
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // The corpus walk
    // -----------------------------------------------------------------------

    /// A staging area of untracked local files, not corpus content.
    const UNTRACKED: &str = "pending";

    /// Root directories that are the corpus repo's machinery rather than specimens.
    /// Everything else at the root is a model directory or `library/`.
    const NOT_SPECIMEN_DIRS: &[&str] = &["nix", "tools"];

    /// Extensions the corpus uses for its own prose, indexes, vendor documentation and
    /// the extracts its tools cut out of oversized originals. None of them is a file a
    /// Nord instrument reads or writes, so none of them belongs in a container sweep.
    ///
    /// ⚠️ This is the sweep's only extension list, and it names what to *leave out*. A
    /// vendor format nobody has met yet is picked up by magic, lands in
    /// [`Class::Unknown`], and trips [`UNKNOWN_CAP`] — which is the point.
    const NOT_SPECIMEN_EXTS: &[&str] = &[
        "bin",       // zip central directories cut out by `corpus zip-extract`
        "body",      // a CBIN body cut out of its container
        "gitignore", //
        "html",      // Clavia's program-list documents
        "json",      // library/library.json
        "md",        // the corpus's own prose
        "pcapng",    // USB captures — nord-usb's material, not a container
        "pdf",       // Clavia's manuals
        "tsv",       // zip member listings
        "txt",       // shape files, and Clavia's sound-card notes
    ];

    /// Every specimen under the corpus root, in a stable order.
    ///
    /// The root's own files and [`NOT_SPECIMEN_DIRS`] are skipped, because every
    /// specimen lives under a model directory or `library/`. Below that the only filter
    /// is [`NOT_SPECIMEN_EXTS`] — what a file *is* comes from its magic, so a new
    /// extension needs no list to be swept.
    fn specimens(root: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut stack: Vec<PathBuf> = Vec::new();
        for entry in fs::read_dir(root).unwrap_or_else(|e| panic!("{}: {e}", root.display())) {
            let path = entry.unwrap().path();
            let name = file_name(&path);
            if path.is_dir() && !name.starts_with('.') && !NOT_SPECIMEN_DIRS.contains(&&*name) {
                stack.push(path);
            }
        }
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
                let path = entry.unwrap().path();
                let name = file_name(&path);
                if path.is_dir() {
                    if name != UNTRACKED && !name.starts_with('.') {
                        stack.push(path);
                    }
                } else if !NOT_SPECIMEN_EXTS.contains(&&*extension(&path)) {
                    found.push(path);
                }
            }
        }
        found.sort();
        found
    }

    fn file_name(path: &Path) -> String {
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }

    /// The model directory a file belongs to — the first component under the corpus
    /// root. `library` is one of them: the sample pool is shared across instruments,
    /// which is why it sits beside them rather than under one.
    fn model(root: &Path, path: &Path) -> String {
        rel(root, path).split('/').next().unwrap_or("?").to_string()
    }

    fn extension(path: &Path) -> String {
        path.extension()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }

    // -----------------------------------------------------------------------
    // The floors
    // -----------------------------------------------------------------------

    /// `(model, extension, at least this many files)` — what the corpus holds.
    ///
    /// A specimen directory that shrinks is a corpus mistake, and it would otherwise
    /// show up as nothing at all: fewer files means fewer trials means green. Measured
    /// against the **git tier**, so the R2-only originals the full corpus splices back
    /// (`nc2`'s `.npip`, the two `ne5` bundles) are absent from these numbers and clear
    /// them by one.
    const FILES: &[(&str, &str, usize)] = &[
        ("library", "nsmp", 7),
        ("library", "nsmp3", 7),
        ("library", "nsmp4", 5),
        ("nc2", "ncpg", 125),
        ("nc2", "ncsy", 1),
        ("nc2d", "nc2p", 126),
        ("nc2d", "nc2s", 1),
        ("nd2", "nd2_bank", 4),
        ("nd3p", "nd3_kitbank", 4),
        ("ne3", "neop", 51),
        ("ne3", "nepg", 128),
        ("ne3hp", "neop", 51),
        ("ne3hp", "nepg", 128),
        ("ne4", "ne4l", 4),
        ("ne4", "ne4p", 128),
        ("ne4", "ne4s", 1),
        ("ne4d", "ne4l", 4),
        ("ne4d", "ne4p", 128),
        ("ne4d", "ne4s", 1),
        ("ne5", "ne5l", 6),
        ("ne5", "ne5p", 828),
        ("ne5", "ne5s", 122),
        ("ne5", "ne5t", 75),
        ("ne5", "nsmp", 32),
        ("ne5", "nsmpproj", 6),
        ("ne6", "ne6l", 8),
        ("ne6", "ne6p", 240),
        ("ne6", "ne6t", 1),
        ("ne7", "ne7l", 5),
        ("ne7", "ne7p", 300),
        ("ne7", "ne7t", 1),
        ("ngrand", "ng2l", 6),
        ("ngrand", "ng2p", 288),
        ("ngrand", "ng2t", 1),
        ("nl", "mid", 17),
        ("nl2", "mid", 17),
        ("nl2x", "mid", 5),
        ("nl2x", "syx", 5),
        ("nl3", "mid", 10),
        ("nl3", "syx", 10),
        ("nl4", "nl4p", 90),
        ("nl4", "nl4s", 297),
        ("nl4", "nl4t", 1),
        ("nla1", "nlap", 150),
        ("nla1", "nlas", 350),
        ("nla1", "nlat", 1),
        ("no3", "no3p", 150),
        ("no3", "no3t", 1),
        ("np", "npli", 5),
        ("np", "nppg", 120),
        ("np", "npsy", 1),
        ("np2", "np2l", 5),
        ("np2", "np2p", 240),
        ("np2", "np2s", 1),
        ("np3", "np3l", 5),
        ("np3", "np3p", 200),
        ("np3", "np3s", 1),
        ("np4", "np4l", 5),
        ("np4", "np4p", 200),
        ("np4", "np4t", 1),
        ("np5", "np5l", 5),
        ("np5", "np5p", 300),
        ("np5", "np5t", 1),
        ("ns2", "ns2l", 5),
        ("ns2", "ns2p", 400),
        ("ns2", "ns2s", 222),
        ("ns2", "ns2y", 1),
        ("ns2ex", "ns2l", 5),
        ("ns2ex", "ns2p", 400),
        ("ns2ex", "ns2s", 222),
        ("ns2ex", "ns2y", 1),
        ("ns3", "ns3f", 300),
        ("ns3", "ns3l", 5),
        ("ns3", "ns3s", 10),
        ("ns3", "ns3t", 1),
        ("ns3", "ns3y", 300),
        ("ns4", "ns4l", 8),
        ("ns4", "ns4n", 96),
        ("ns4", "ns4o", 64),
        ("ns4", "ns4p", 384),
        ("ns4", "ns4t", 1),
        ("ns4", "ns4y", 384),
        ("nsclassic", "nspg", 126),
        ("nsclassic", "nss", 297),
        ("nsex", "nspg", 126),
        ("nsex", "nss", 297),
        ("nw", "nwp", 1024),
        ("nw", "nwsy", 1),
        ("nw2", "nw2l", 5),
        ("nw2", "nw2p", 350),
        ("nw2", "nw2s", 1),
    ];

    /// `(model, outcome, at least this many files reach it)` — how far this build gets.
    ///
    /// **Ratchets up only.** Teaching the crate a format moves files from `container` to
    /// `decoded` and the numbers here follow; a file going the other way is a
    /// regression, and the fix is the code, never this table.
    ///
    /// A model with no CBIN in it — the four Leads, the two Drums — has no row: its
    /// files are floored by [`CLASSES`] instead, where they cannot be read as a claim
    /// about decoding progress.
    const OUTCOMES: &[(&str, Outcome, usize)] = &[
        ("library", Outcome::Container, 19),
        // The `.nsmp` v2 specimens. The v3 and v4 generations carry the same `nsmp` tag
        // and a section chain this build misreads, so they stop at `container`.
        ("library", Outcome::Decoded, 7),
        ("nc2", Outcome::Container, 126),
        ("nc2d", Outcome::Container, 127),
        ("ne3", Outcome::Container, 179),
        ("ne3hp", Outcome::Container, 179),
        ("ne4", Outcome::Container, 133),
        ("ne4d", Outcome::Container, 133),
        ("ne5", Outcome::Decoded, 1063),
        ("ne6", Outcome::Container, 249),
        ("ne7", Outcome::Container, 306),
        ("ngrand", Outcome::Container, 295),
        ("nl4", Outcome::Container, 388),
        ("nla1", Outcome::Container, 501),
        ("no3", Outcome::Container, 151),
        ("np", Outcome::Container, 126),
        ("np2", Outcome::Container, 246),
        ("np3", Outcome::Container, 206),
        ("np4", Outcome::Container, 206),
        ("np5", Outcome::Container, 306),
        ("ns2", Outcome::Container, 628),
        ("ns2ex", Outcome::Container, 628),
        ("ns3", Outcome::Container, 616),
        ("ns4", Outcome::Container, 937),
        ("nsclassic", Outcome::Container, 423),
        ("nsex", Outcome::Container, 423),
        ("nw", Outcome::Container, 1025),
        ("nw2", Outcome::Container, 356),
    ];

    /// `(class, at least this many files, the outcome every one of them reaches)` —
    /// corpus-wide, across every model.
    ///
    /// The class counts are a property of the material and only grow. The outcome is
    /// the stronger claim: **every** file of the class reaches it, so a single `.mid`
    /// this crate started returning an entity for, or a single type-0 file whose crc16
    /// stopped verifying, fails here and names itself.
    ///
    /// `cbin-type0` is what keeps the short header exercised. The ne5 factory banks, the
    /// whole Stage 2 export, the Stage EX, the Wave, both C2s and both Leads that have
    /// CBIN at all are type 0; losing them would remove the only evidence it is read
    /// correctly.
    const CLASSES: &[(&str, usize, Outcome)] = &[
        ("cbin-type0", 5695, Outcome::Container),
        ("cbin-type1", 4280, Outcome::Container),
        ("midi", 49, Outcome::Unsupported),
        ("smac", 6, Outcome::Unsupported),
        ("sysex", 15, Outcome::Unsupported),
        ("zip", 8, ZIP_OUTCOME),
    ];

    /// ⚠️ **The one outcome that depends on how cargo was invoked.** `nord-cli` takes
    /// `nord-format` with `bundle`, so a `--workspace` build unifies it on and every ZIP
    /// in the corpus reads as a bundle; `cargo test -p nord-format --features corpus` —
    /// which is what the Nix check runs — leaves it off and the same files are refused.
    /// Both are correct, and a floor written for one of them would fail the other.
    const ZIP_OUTCOME: Outcome = if cfg!(feature = "bundle") {
        Outcome::Container
    } else {
        Outcome::Unsupported
    };

    /// At most this many files whose magic names nothing.
    ///
    /// ⚠️ **A cap, not a floor** — the one number here that must never go up. The
    /// residue today is the `meta.xml` manifest inside each vendor backup and bundle
    /// archive, one per extracted archive. Anything else arriving unidentified is a
    /// format nobody has looked at, and this is what says so.
    const UNKNOWN_CAP: usize = 15;

    /// Below this many specimens the corpus root is not a corpus root. The git tier
    /// holds ten thousand and the full corpus another 2,433 on top.
    const CORPUS_FLOOR: usize = 9000;

    // -----------------------------------------------------------------------
    // The sample pool
    // -----------------------------------------------------------------------

    /// `(pool extension, outcome, at least this many files reach it)`, measured against
    /// the full 2,433-file pool (`nix build .#nord-corpus-full`).
    ///
    /// **Ratchets up only**, same rule as [`OUTCOMES`] — `library.json` cannot supply
    /// these numbers, since it records what the pool *is*, not what this build can do
    /// with it. What it does supply is the completeness floors in [`pool_coverage`].
    const POOL_OUTCOMES: &[(&str, Outcome, usize)] = &[
        ("cn3", Outcome::Unsupported, 14),
        ("nsmp", Outcome::Decoded, 772),
        ("nsmp3", Outcome::Container, 1081),
        ("nsmp4", Outcome::Container, 557),
        ("nsp", Outcome::Container, 9),
    ];

    fn library_pool_dir(root: &Path) -> PathBuf {
        root.join("library").join("pool")
    }

    /// `library/library.json`'s pool totals, read fresh each run rather than copied into
    /// this file — a pool that grows updates its own floor, instead of this suite
    /// silently accepting fewer files than the index claims.
    ///
    /// The index covers the whole R2 tier, so the pool's own counts are `totals.pool`.
    /// `totals.files` is the wider number: it also counts the recorded artifacts, which
    /// assemble at their capture paths and never under `library/pool`.
    fn pool_totals(root: &Path) -> serde_json::Value {
        let path = root.join("library").join("library.json");
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let index: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let totals = index["totals"]["pool"].clone();
        assert!(
            totals.is_object(),
            "{}: no totals.pool object — the index schema moved and these floors are \
             reading nothing",
            path.display(),
        );
        totals
    }

    /// `(extension, files)` from one of `totals.pool`'s breakdowns.
    fn pool_rows(totals: &serde_json::Value, key: &str) -> Vec<(String, usize)> {
        totals[key]
            .as_object()
            .unwrap_or_else(|| panic!("totals.pool.{key}"))
            .iter()
            .map(|(name, row)| {
                let files = row["files"]
                    .as_u64()
                    .unwrap_or_else(|| panic!("totals.pool.{key}[{name}].files"));
                (name.clone(), files as usize)
            })
            .collect()
    }

    /// The pool's filesystem extension for one `library.json` generation — `nsmp` for
    /// v2, `nsmp<generation>` for v3/v4. Confirmed against the index's own rows.
    fn pool_extension(generation: &str) -> String {
        if generation == "2" {
            "nsmp".to_string()
        } else {
            format!("nsmp{generation}")
        }
    }

    /// Trials scoped to `library/pool` alone — not the specimens the general sweep
    /// already counts under the same `library` model bucket, since the pool physically
    /// duplicates 19 of them under a second path and a floor here must not double-count.
    fn pool_coverage(root: &Path, c: &std::sync::Arc<Census>) -> Vec<Trial> {
        let totals = pool_totals(root);
        let expected_total = totals["files"].as_u64().expect("totals.pool.files") as usize;
        let by_extension = pool_rows(&totals, "by_extension");
        let by_generation = pool_rows(&totals, "by_generation");

        let mut trials = Vec::new();

        {
            let c = c.clone();
            trials.push(aggregate(
                "coverage/library_pool/files/total".to_string(),
                move || {
                    let found: usize = c.pool_by_extension.values().sum();
                    assert!(
                        found >= expected_total,
                        "library/pool holds {found} files, library.json claims \
                         {expected_total} — the nix assembly and the index have drifted",
                    );
                },
            ));
        }

        // `by_extension` covers the whole pool; `by_generation` covers only the rows
        // that have a generation, which is the `.nsmp*` lineage alone — `.nsp` and
        // `.cn3` are not points on it and carry a null. Checking the two against each
        // other is what notices the index changing shape under these floors.
        {
            let lineage: usize = by_extension
                .iter()
                .filter(|(ext, _)| ext.starts_with("nsmp"))
                .map(|(_, n)| n)
                .sum();
            let generations: usize = by_generation.iter().map(|(_, n)| n).sum();
            let named: Vec<String> = by_generation.iter().map(|(g, _)| g.clone()).collect();
            trials.push(aggregate(
                "coverage/library_pool/index_shape".to_string(),
                move || {
                    assert_eq!(
                        generations, lineage,
                        "library.json: generations {named:?} account for {generations} pool \
                         files but the .nsmp* extensions hold {lineage} — by_generation is \
                         supposed to cover that lineage and nothing else",
                    );
                },
            ));
        }

        for (ext, expected) in by_extension {
            let c = c.clone();
            let name = format!("coverage/library_pool/files/{ext}");
            trials.push(aggregate(name, move || {
                let found = c.pool_by_extension.get(&ext).copied().unwrap_or(0);
                assert!(
                    found >= expected,
                    "library/pool holds {found} .{ext} files, library.json claims \
                     {expected} — a pool file went missing",
                );
            }));
        }

        for (generation, expected) in by_generation {
            let c = c.clone();
            let ext = pool_extension(&generation);
            let name = format!("coverage/library_pool/generation/v{generation}");
            trials.push(aggregate(name, move || {
                let found = c.pool_by_extension.get(&ext).copied().unwrap_or(0);
                assert!(
                    found >= expected,
                    "library/pool holds {found} .{ext} files, library.json claims \
                     {expected} for generation {generation} — a pool file went missing",
                );
            }));
        }

        for &(ext, outcome, floor) in POOL_OUTCOMES {
            let c = c.clone();
            let name = format!("coverage/library_pool/outcome/{ext}/{}", outcome.as_str());
            trials.push(aggregate(name, move || {
                let found: usize = c
                    .pool_by_outcome
                    .iter()
                    .filter(|((e, o), _)| e == ext && *o >= outcome)
                    .map(|(_, n)| n)
                    .sum();
                assert!(
                    found >= floor,
                    "library/pool: {found} .{ext} files reach `{}`, expected at least \
                     {floor} — support ratchets up, so this is a regression, not a floor \
                     to lower",
                    outcome.as_str(),
                );
            }));
        }

        trials
    }

    // -----------------------------------------------------------------------
    // The harness
    // -----------------------------------------------------------------------

    pub fn run() -> ! {
        let args = Arguments::from_args();
        let root = corpus_dir();
        let files = specimens(&root);
        assert!(
            files.len() >= CORPUS_FLOOR,
            "found only {} specimens under {} — is this a nord-corpus checkout root?",
            files.len(),
            root.display(),
        );

        let mut trials: Vec<Trial> = files
            .iter()
            .map(|path| {
                let name = format!("container/{}", rel(&root, path));
                let at = path.clone();
                Trial::test(name, move || {
                    check(at);
                    Ok(())
                })
            })
            .collect();

        let pool_dir = library_pool_dir(&root);
        let c = std::sync::Arc::new(census(&root, &pool_dir, &files));
        trials.extend(coverage(&c));

        if pool_dir.is_dir() {
            trials.extend(pool_coverage(&root, &c));
        } else {
            let at = root.clone();
            trials.push(
                Trial::test("library_pool/absent".to_string(), move || {
                    println!(
                        "library/pool absent under {} — this corpus build has no sample \
                         pool spliced in (nix build .#nord-corpus-full, not plain \
                         nord-corpus)",
                        at.display(),
                    );
                    Ok(())
                })
                .with_ignored_flag(true),
            );
        }

        libtest_mimic::run(&args, trials).exit()
    }

    fn aggregate(name: String, check: impl FnOnce() + Send + 'static) -> Trial {
        Trial::test(name, move || {
            check();
            Ok(())
        })
    }

    /// One pass over the corpus, kept so every aggregate below reads the same numbers.
    struct Census {
        /// `(model, extension) -> files`.
        by_extension: BTreeMap<(String, String), usize>,
        /// `(model, outcome) -> files`.
        by_outcome: BTreeMap<(String, Outcome), usize>,
        /// `class name -> files`.
        by_class: BTreeMap<String, usize>,
        /// `class name -> the worst outcome any file of it reached, and one to look at`.
        worst_of_class: BTreeMap<String, (Outcome, String, String)>,
        /// `extension -> files`, under `library/pool` alone.
        pool_by_extension: BTreeMap<String, usize>,
        /// `(extension, outcome) -> files`, under `library/pool` alone.
        pool_by_outcome: BTreeMap<(String, Outcome), usize>,
        /// The worst class each model reached, with one file to look at.
        examples: BTreeMap<(String, Outcome), (String, String)>,
    }

    fn census(root: &Path, pool_dir: &Path, files: &[PathBuf]) -> Census {
        let mut c = Census {
            by_extension: BTreeMap::new(),
            by_outcome: BTreeMap::new(),
            by_class: BTreeMap::new(),
            worst_of_class: BTreeMap::new(),
            pool_by_extension: BTreeMap::new(),
            pool_by_outcome: BTreeMap::new(),
            examples: BTreeMap::new(),
        };
        for path in files {
            let bytes = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let model = model(root, path);
            let ext = extension(path);
            let (class, outcome, why) = classify(path, &bytes);

            *c.by_extension
                .entry((model.clone(), ext.clone()))
                .or_default() += 1;
            *c.by_outcome.entry((model.clone(), outcome)).or_default() += 1;
            *c.by_class.entry(class.name()).or_default() += 1;
            c.worst_of_class
                .entry(class.name())
                .and_modify(|worst| {
                    if outcome < worst.0 {
                        *worst = (outcome, rel(root, path), why.clone());
                    }
                })
                .or_insert_with(|| (outcome, rel(root, path), why.clone()));
            c.examples
                .entry((model, outcome))
                .or_insert_with(|| (rel(root, path), why));

            if path.starts_with(pool_dir) {
                *c.pool_by_extension.entry(ext.clone()).or_default() += 1;
                *c.pool_by_outcome.entry((ext, outcome)).or_default() += 1;
            }
        }
        c
    }

    fn coverage(c: &std::sync::Arc<Census>) -> Vec<Trial> {
        let mut trials = Vec::new();

        for &(model, ext, floor) in FILES {
            let c = c.clone();
            trials.push(aggregate(
                format!("coverage/files/{model}/{ext}"),
                move || {
                    let found = c
                        .by_extension
                        .get(&(model.to_string(), ext.to_string()))
                        .copied()
                        .unwrap_or(0);
                    assert!(
                        found >= floor,
                        "{model} holds {found} .{ext} files, expected at least {floor} — a \
                     specimen directory shrank",
                    );
                },
            ));
        }

        for &(model, outcome, floor) in OUTCOMES {
            let c = c.clone();
            let name = format!("coverage/outcome/{model}/{}", outcome.as_str());
            trials.push(aggregate(name, move || {
                // "At least this class", so `decoded` counts towards `container` too:
                // teaching the crate a format must never trip a floor written for the
                // weaker claim.
                let found: usize = c
                    .by_outcome
                    .iter()
                    .filter(|((m, o), _)| m == model && *o >= outcome)
                    .map(|(_, n)| n)
                    .sum();
                let example = c
                    .examples
                    .iter()
                    .find(|((m, o), _)| m == model && *o < outcome)
                    .map(|(_, (path, why))| format!(" (e.g. {path}: {why})"))
                    .unwrap_or_default();
                assert!(
                    found >= floor,
                    "{model}: {found} files reach `{}`, expected at least {floor}{example} \
                     — support ratchets up, so this is a regression, not a floor to lower",
                    outcome.as_str(),
                );
            }));
        }

        for &(class, floor, outcome) in CLASSES {
            let c = c.clone();
            trials.push(aggregate(format!("coverage/class/{class}"), move || {
                let found = c.by_class.get(class).copied().unwrap_or(0);
                assert!(
                    found >= floor,
                    "{found} `{class}` files, expected at least {floor} — specimens of \
                     that container went missing",
                );
                let (worst, path, why) = c
                    .worst_of_class
                    .get(class)
                    .expect("a class with files has a worst outcome");
                assert!(
                    *worst >= outcome,
                    "`{class}` reaches `{}` at worst, expected every file to reach `{}` \
                     ({path}: {why})",
                    worst.as_str(),
                    outcome.as_str(),
                );
            }));
        }

        {
            let c = c.clone();
            trials.push(aggregate("coverage/class/unknown".to_string(), move || {
                let found = c.by_class.get("unknown").copied().unwrap_or(0);
                let example = c
                    .worst_of_class
                    .get("unknown")
                    .map(|(_, path, why)| format!(" (e.g. {path}: {why})"))
                    .unwrap_or_default();
                assert!(
                    found <= UNKNOWN_CAP,
                    "{found} files have a magic this sweep does not know, at most \
                     {UNKNOWN_CAP} expected{example} — a container nobody has looked at \
                     is in the corpus, and this is the cap that says so",
                );
            }));
        }

        // Not an assertion: the table a reader wants when they ask what this build can
        // do with the corpus. Printed by a named trial so it is in every run's output.
        let c = c.clone();
        trials.push(aggregate("report".to_string(), move || {
            print!("{}", report(&c));
        }));

        trials
    }

    fn report(c: &Census) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "\n{:10} {:>8} {:>10} {:>12} {:>13} {:>8}  extensions",
            "model", "decoded", "container", "unsupported", "unidentified", "refused"
        );
        let models: std::collections::BTreeSet<&String> =
            c.by_outcome.keys().map(|(m, _)| m).collect();
        for model in models {
            let n = |o: Outcome| c.by_outcome.get(&(model.clone(), o)).copied().unwrap_or(0);
            let exts: Vec<String> = c
                .by_extension
                .iter()
                .filter(|((m, _), _)| m == model)
                .map(|((_, e), n)| format!("{e}:{n}"))
                .collect();
            let _ = writeln!(
                out,
                "{model:10} {:>8} {:>10} {:>12} {:>13} {:>8}  {}",
                n(Outcome::Decoded),
                n(Outcome::Container),
                n(Outcome::Unsupported),
                n(Outcome::Unidentified),
                n(Outcome::Refused),
                exts.join(" "),
            );
        }
        let _ = writeln!(out, "\ncontainer classes:");
        for (class, n) in &c.by_class {
            let worst = c
                .worst_of_class
                .get(class)
                .map(|(o, _, _)| o.as_str())
                .unwrap_or("-");
            let _ = writeln!(out, "  {class:12} {n:>6}  worst outcome: {worst}");
        }
        if !c.pool_by_extension.is_empty() {
            let _ = writeln!(out, "\nlibrary/pool:");
            for ((ext, outcome), n) in &c.pool_by_outcome {
                let _ = writeln!(out, "  {ext:8} {:12} {n:>6}", outcome.as_str());
            }
        }
        out
    }
}

fn main() {
    #[cfg(feature = "corpus")]
    containers::run();
}
