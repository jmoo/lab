//! Every CBIN file the corpus ships, classified at the container level.
//!
//! The corpus covers seven instruments and a shared sample library, and this build has
//! body schemas for one of them. That is not a reason to look at only one: **the
//! container is common to all of them**, and a sweep over every model checks the part
//! that is shared, counts what is not, and turns "we do not read `.ns4p`" from a silent
//! absence into a number that cannot quietly get worse.
//!
//! Three outcomes, worst to best — see [`Outcome`]:
//!
//! * `Refused` — the container itself does not read.
//! * `Container` — the container reads and its checksum verifies; the body is one
//!   unknown region, because this build has no schema for that tag.
//! * `Decoded` — a schema accounts for the body, and re-emitting reproduces the file
//!   byte for byte.
//!
//! ⚠️ **An unknown format is a classification, not a failure.** A `.ns4p` landing in
//! `Container` is the honest answer and the suite stays green; what fails is a file
//! sliding *down* a class, which the ratcheted floors in [`FILES`] and [`OUTCOMES`]
//! catch. Never lower a floor to make a run green — a floor going down is the finding.
//!
//! `library/pool` — the 1,018-file vendor sample pool `mkCorpus { library = true; }`
//! splices in — gets the same per-file sweep automatically (nothing excludes it from the
//! walk), plus completeness floors against the corpus's own `library.json` in
//! [`pool_coverage`]. A plain corpus has no pool at all; that is not silently fewer
//! trials, it is one visible ignored trial saying so.
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

    /// How far this build gets on one file. Ordered worst to best, so a floor reads as
    /// "at least this many files reach at least this class".
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
    pub enum Outcome {
        /// The container does not read: not a CBIN file, truncated, a header generation
        /// with no known layout, or a checksum that does not hold.
        Refused,
        /// The container reads and its checksum verifies. The body is one unknown
        /// region — this build has no schema for the tag, or refuses its version.
        Container,
        /// A schema accounts for the body and the file re-emits byte for byte.
        Decoded,
    }

    impl Outcome {
        fn as_str(self) -> &'static str {
            match self {
                Outcome::Refused => "refused",
                Outcome::Container => "container",
                Outcome::Decoded => "decoded",
            }
        }
    }

    /// Classify one file, with the reason in the reader's words.
    fn classify(path: &Path, bytes: &[u8]) -> (Outcome, String) {
        if let Err(e) = container::widen(bytes) {
            return (Outcome::Refused, e.to_string());
        }
        match nord_format::from_path(path) {
            Ok(_) => (Outcome::Decoded, String::new()),
            Err(e) => (Outcome::Container, e.to_string()),
        }
    }

    /// One file's trial: say what class it lands in, and hold it to that class's
    /// contract. Only `Decoded` has one — that the file re-emits byte for byte.
    fn check(path: PathBuf) {
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        match classify(&path, &bytes) {
            (Outcome::Refused, why) => println!("refused at the container: {why}"),
            (Outcome::Container, why) => println!("container reads, body unknown: {why}"),
            (Outcome::Decoded, _) => {
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

    /// Every CBIN file under the corpus root, in a stable order.
    ///
    /// Sniffed by magic rather than by extension: the corpus carries twenty-odd
    /// extensions across seven instruments and the point of this sweep is that a new one
    /// needs no list to be picked up.
    fn cbin_files(root: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
                let path = entry.unwrap().path();
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                if path.is_dir() {
                    if name != UNTRACKED && !name.starts_with('.') {
                        stack.push(path);
                    }
                } else if is_cbin(&path) {
                    found.push(path);
                }
            }
        }
        found.sort();
        found
    }

    fn is_cbin(path: &Path) -> bool {
        let Ok(file) = fs::File::open(path) else {
            return false;
        };
        use std::io::Read;
        let mut magic = [0u8; 4];
        file.take(4).read_exact(&mut magic).is_ok() && &magic == b"CBIN"
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
    /// show up as nothing at all: fewer files means fewer trials means green.
    const FILES: &[(&str, &str, usize)] = &[
        ("library", "nsmp", 7),
        ("library", "nsmp3", 7),
        ("library", "nsmp4", 5),
        ("ne5", "ne5l", 6),
        ("ne5", "ne5p", 828),
        ("ne5", "ne5s", 122),
        ("ne5", "ne5t", 75),
        ("ne5", "nsmp", 32),
        ("ne6", "ne6l", 8),
        ("ne6", "ne6p", 240),
        ("ne6", "ne6t", 1),
        ("ne7", "ne7l", 5),
        ("ne7", "ne7p", 300),
        ("ne7", "ne7t", 1),
        ("ns2", "ns2l", 5),
        ("ns2", "ns2p", 400),
        ("ns2", "ns2s", 222),
        ("ns2", "ns2y", 1),
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
        ("nsex", "nspg", 126),
        ("nsex", "nss", 297),
    ];

    /// `(model, outcome, at least this many files reach it)` — how far this build gets.
    ///
    /// **Ratchets up only.** Teaching the crate a format moves files from `container` to
    /// `decoded` and the numbers here follow; a file going the other way is a
    /// regression, and the fix is the code, never this table.
    const OUTCOMES: &[(&str, Outcome, usize)] = &[
        ("library", Outcome::Container, 19),
        // The `.nsmp` v2 specimens. The v3 and v4 generations carry the same `nsmp` tag
        // and a section chain this build misreads, so they stop at `container`.
        ("library", Outcome::Decoded, 7),
        ("ne5", Outcome::Decoded, 1063),
        ("ne6", Outcome::Container, 249),
        ("ne7", Outcome::Container, 306),
        ("ns2", Outcome::Container, 628),
        ("ns3", Outcome::Container, 616),
        ("ns4", Outcome::Container, 937),
        ("nsex", Outcome::Container, 423),
    ];

    /// At least this many type-0 files, corpus-wide.
    ///
    /// The generation is a property of the material, not of this build: the ne5 factory
    /// banks, the whole Stage 2 export, the Stage EX and half the sample library are
    /// type 0. Losing them would remove the only evidence the short header is read
    /// correctly at all.
    const TYPE_0_FLOOR: usize = 1275;

    // -----------------------------------------------------------------------
    // The sample pool
    // -----------------------------------------------------------------------

    /// `(pool extension, outcome, at least this many files reach it)`, measured against
    /// the full 1,018-file pool (`nix build .#nord-corpus-full` with `library = true`).
    ///
    /// **Ratchets up only**, same rule as [`OUTCOMES`] — library.json cannot supply these
    /// numbers, since it records what the pool *is*, not what this build can do with it.
    ///
    /// All 268 `.nsmp` (v2) decode. All 470 `.nsmp3` (v3) and 280 `.nsmp4` (v4) stop at
    /// `container` — this build has no schema for the v3/v4 sample body. Nothing in the
    /// pool is `Refused`.
    const POOL_OUTCOMES: &[(&str, Outcome, usize)] = &[
        ("nsmp", Outcome::Decoded, 268),
        ("nsmp3", Outcome::Container, 470),
        ("nsmp4", Outcome::Container, 280),
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
    fn pool_coverage(root: &Path, pool_dir: &Path, files: &[PathBuf]) -> Vec<Trial> {
        let totals = pool_totals(root);
        let expected_total = totals["files"].as_u64().expect("totals.pool.files") as usize;
        let expected_by_generation: Vec<(String, usize)> = totals["by_generation"]
            .as_object()
            .expect("totals.pool.by_generation")
            .iter()
            .map(|(gen, row)| {
                let files = row["files"]
                    .as_u64()
                    .expect("totals.pool.by_generation[].files");
                (gen.clone(), files as usize)
            })
            .collect();

        let mut by_extension: BTreeMap<String, usize> = BTreeMap::new();
        let mut by_outcome: BTreeMap<(String, Outcome), usize> = BTreeMap::new();
        let mut found_total = 0usize;
        for path in files.iter().filter(|p| p.starts_with(pool_dir)) {
            found_total += 1;
            let bytes = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let ext = extension(path);
            let (outcome, _) = classify(path, &bytes);
            *by_extension.entry(ext.clone()).or_default() += 1;
            *by_outcome.entry((ext, outcome)).or_default() += 1;
        }
        let by_extension = std::sync::Arc::new(by_extension);
        let by_outcome = std::sync::Arc::new(by_outcome);

        let mut trials = Vec::new();

        trials.push(aggregate(
            "coverage/library_pool/files/total".to_string(),
            move || {
                assert!(
                    found_total >= expected_total,
                    "library/pool holds {found_total} files, library.json claims \
                     {expected_total} — the nix assembly and the index have drifted",
                );
            },
        ));

        for (generation, expected) in expected_by_generation {
            let ext = pool_extension(&generation);
            let by_extension = by_extension.clone();
            let name = format!("coverage/library_pool/files/v{generation}");
            trials.push(aggregate(name, move || {
                let found = by_extension.get(&ext).copied().unwrap_or(0);
                assert!(
                    found >= expected,
                    "library/pool holds {found} .{ext} files, library.json claims \
                     {expected} for generation {generation} — a pool file went missing",
                );
            }));
        }

        for &(ext, outcome, floor) in POOL_OUTCOMES {
            let by_outcome = by_outcome.clone();
            let name = format!("coverage/library_pool/outcome/{ext}/{}", outcome.as_str());
            trials.push(aggregate(name, move || {
                let found: usize = by_outcome
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
        let files = cbin_files(&root);
        assert!(
            files.len() > 4000,
            "found only {} CBIN files under {} — is this a nord-corpus checkout root?",
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

        trials.extend(coverage(&root, &files));

        let pool_dir = library_pool_dir(&root);
        if pool_dir.is_dir() {
            trials.extend(pool_coverage(&root, &pool_dir, &files));
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
        /// `header type -> files`.
        by_generation: BTreeMap<u32, usize>,
        /// The worst class each model reached, with one file to look at.
        examples: BTreeMap<(String, Outcome), (String, String)>,
    }

    fn census(root: &Path, files: &[PathBuf]) -> Census {
        let mut c = Census {
            by_extension: BTreeMap::new(),
            by_outcome: BTreeMap::new(),
            by_generation: BTreeMap::new(),
            examples: BTreeMap::new(),
        };
        for path in files {
            let bytes = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let model = model(root, path);
            let (outcome, why) = classify(path, &bytes);
            *c.by_extension
                .entry((model.clone(), extension(path)))
                .or_default() += 1;
            *c.by_outcome.entry((model.clone(), outcome)).or_default() += 1;
            if let Ok(generation) = container::header_type(&bytes) {
                *c.by_generation.entry(generation).or_default() += 1;
            }
            c.examples
                .entry((model, outcome))
                .or_insert_with(|| (rel(root, path), why));
        }
        c
    }

    fn coverage(root: &Path, files: &[PathBuf]) -> Vec<Trial> {
        let c = std::sync::Arc::new(census(root, files));
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

        {
            let c = c.clone();
            trials.push(aggregate("coverage/generations".to_string(), move || {
                let found = c
                    .by_generation
                    .get(&container::TYPE_SHORT)
                    .copied()
                    .unwrap_or(0);
                assert!(
                    found >= TYPE_0_FLOOR,
                    "{found} type-0 files, expected at least {TYPE_0_FLOOR} — the short \
                     header would go unexercised",
                );
            }));
        }

        // Not an assertion: the table a reader wants when they ask what this build can
        // do with the corpus. Printed by a named trial so it is in every run's output.
        trials.push(aggregate("report".to_string(), move || {
            print!("{}", report(&c));
        }));

        trials
    }

    fn report(c: &Census) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "\n{:8} {:>8} {:>10} {:>9}  extensions",
            "model", "decoded", "container", "refused"
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
                "{model:8} {:>8} {:>10} {:>9}  {}",
                n(Outcome::Decoded),
                n(Outcome::Container),
                n(Outcome::Refused),
                exts.join(" "),
            );
        }
        let _ = writeln!(out, "\nCBIN header generations: {:?}", c.by_generation);
        out
    }
}

fn main() {
    #[cfg(feature = "corpus")]
    containers::run();
}
