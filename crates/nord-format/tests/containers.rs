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
