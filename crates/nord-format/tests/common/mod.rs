//! What every corpus test target needs: the corpus root, and one program's decode as
//! comparable rows.
//!
//! ⚠️ A rustc-visible module, not a test target — each corpus test compiles its own copy,
//! so an item no single target uses is still live.
#![allow(dead_code)]

use nord_format::electro5::program::OrganPanel;
use nord_format::electro5::{OrganModel, Program};
use nord_format::panel::Panel;
use nord_format::{electro5, Entity};
use std::fs;
use std::path::{Path, PathBuf};

/// Root of the specimen corpus, taken from `NORD_CORPUS_DIR` — **the whole corpus**,
/// the directory holding `ne5/`, `ne6/`, … and `library/`, not one model.
///
/// Since these tests only compile under the `corpus` feature, a missing
/// `NORD_CORPUS_DIR` is a hard error, not a skip.
pub fn corpus_dir() -> PathBuf {
    let dir: PathBuf = std::env::var_os("NORD_CORPUS_DIR")
        .expect("set NORD_CORPUS_DIR to a nord-corpus checkout root for --features corpus")
        .into();
    check_revision(&dir);
    dir
}

/// The Electro 5 model directory.
///
/// Everything with a filename oracle is Electro 5, so the oracle suites join this rather
/// than treating the corpus root as a model root. A suite that walks every model takes
/// [`corpus_dir`] instead.
pub fn ne5_dir() -> PathBuf {
    corpus_dir().join("ne5")
}

/// Refuse a checkout that is not at the revision `crates/corpus_rev.txt` pins, naming
/// which way it is skewed.
///
/// A specimen sweep is only as pinned as the specimens: a checkout at another revision
/// produces failures that read as decode regressions and are not.
///
/// One situation passes silently: a directory git cannot answer for is the Nix store
/// assembly, which has no `.git` at all — the corpus in hand there *is* the pinned one
/// by construction. A **worktree** root does answer git — its `.git` is a file rather
/// than a directory, which `git -C` resolves the same way — so a worktree of the corpus
/// is held to the pin like any other checkout.
fn check_revision(dir: &Path) {
    let pinned = pinned_rev();
    let Some(head) = git(dir, &["rev-parse", "HEAD"]) else {
        return;
    };
    if head == pinned {
        return;
    }

    let ancestor = |older: &str, newer: &str| {
        git(dir, &["merge-base", "--is-ancestor", older, newer]).is_some()
    };
    let fix = if ancestor(&pinned, &head) {
        "the checkout is ahead — bump the pin in crates/corpus_rev.txt"
    } else if ancestor(&head, &pinned) {
        "the checkout is behind — git -C … fetch && git -C … checkout the expected rev"
    } else {
        "the two have diverged, or the expected commit is not in this checkout — fetch it"
    };
    panic!("corpus at {head}, tests expect {pinned} — {fix}");
}

/// The corpus revision this workspace is pinned to, read out of `crates/corpus_rev.txt`
/// — the single file the flake and this guard both read, so there is no second copy to
/// drift.
///
/// Present in every context this guard runs, including the Nix sandbox: `mkRustCrate`
/// hands the build `crates/` (not the flake root) as source, and this file lives inside
/// it. A missing or malformed file panics naming the path — silently disabling the
/// guard on a bad file would be worse than not having one.
fn pinned_rev() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../corpus_rev.txt");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let rev = text.trim();
    if rev.len() != 40 || !rev.bytes().all(|b| b.is_ascii_hexdigit()) {
        panic!(
            "{} does not hold exactly one 40-hex-digit revision, got {rev:?}",
            path.display(),
        );
    }
    rev.to_string()
}

/// `git -C dir <args>`, or `None` if git is absent, the directory is not a checkout, or
/// the command reports failure.
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// One field's decode: a panel-qualified key, where its bits sit, the bits themselves,
/// and what they decoded to.
pub struct Row {
    pub key: String,
    pub placement: String,
    /// The field's bits shifted down to bit 0, carrying no type — so this survives a
    /// field being retyped and pins the placement on its own. `None` where the bits are
    /// not reachable: some organ accessors return a decoded value with no way to ask for
    /// the pattern behind it.
    pub raw: Option<u64>,
    pub value: String,
}

impl Row {
    fn new(
        key: String,
        placement: impl Into<String>,
        raw: impl Into<Option<u64>>,
        value: impl Into<String>,
    ) -> Row {
        Row {
            key,
            placement: placement.into(),
            raw: raw.into(),
            value: value.into(),
        }
    }

    pub fn raw_str(&self) -> String {
        match self.raw {
            Some(raw) => raw.to_string(),
            None => "—".to_string(),
        }
    }
}

/// Every field of a `#[bitpanel]` panel, in declaration order.
pub fn packed<P: Panel>(p: &P) -> Vec<Row> {
    p.field_values()
        .into_iter()
        .map(|f| {
            Row::new(
                format!("{}.{}", P::NAME, f.name),
                f.placement,
                f.raw,
                f.value,
            )
        })
        .collect()
}

/// Where one organ model's state sits, as absolute Electro 5 file offsets.
///
/// Restated here rather than read from `nord-format`: the panel's own copy of these
/// offsets is what is under test, and a snapshot compared against the numbers it came
/// from would pin nothing.
struct Bytes {
    model: OrganModel,
    /// Nine-nibble drawbar block, preset 1 then preset 2.
    drawbars: (usize, usize),
    /// Holds the selected preset in bit 6.
    preset: usize,
    /// Per-preset vibrato/percussion byte. Pipe has neither.
    effect: Option<(usize, usize)>,
    /// Holds the model's vib/chorus type in bits 7..5, shared across presets.
    vib_type: Option<usize>,
}

const ORGAN_BYTES: [Bytes; 4] = [
    Bytes {
        model: OrganModel::B3,
        drawbars: (0x55, 0x5c),
        preset: 0x53,
        effect: Some((0x59, 0x60)),
        vib_type: Some(0x51),
    },
    Bytes {
        model: OrganModel::Vox,
        drawbars: (0x67, 0x6d),
        preset: 0x65,
        effect: Some((0x6b, 0x71)),
        vib_type: Some(0x63),
    },
    Bytes {
        model: OrganModel::Farfisa,
        drawbars: (0x77, 0x7d),
        preset: 0x75,
        effect: Some((0x7b, 0x81)),
        vib_type: Some(0x73),
    },
    Bytes {
        model: OrganModel::Pipe,
        drawbars: (0x87, 0x8d),
        preset: 0x85,
        effect: None,
        vib_type: None,
    },
];

/// The organ panel through its accessors, in the same shape as [`packed`].
///
/// Hand-written on purpose: walking the panel's own [`Panel`] metadata would pin the
/// declaration against itself. The accessors and the offsets above are a second,
/// independent statement of where the organ's state lives.
pub fn organ(o: &OrganPanel) -> Vec<Row> {
    let mut rows = Vec::new();
    let key = |what: &str| format!("OrganPanel.{what}");

    for b in ORGAN_BYTES {
        let model = b.model;

        for (preset, at) in [(1u8, b.drawbars.0), (2, b.drawbars.1)] {
            // No raw column: the nibbles are stored identity, so the decoded array *is*
            // the bits.
            rows.push(Row::new(
                key(&format!("drawbars({model:?},{preset})")),
                format!("{:#04x}..{:#04x}", at, at + 4),
                None,
                format!("{:?}", o.drawbars(model, preset)),
            ));
        }

        rows.push(Row::new(
            key(&format!("preset({model:?})")),
            format!("{:#04x}[6:6]", b.preset),
            u64::from(o.preset(model) == 2),
            o.preset(model).to_string(),
        ));

        // The 3-bit type indexes a per-model table and the index itself is not reachable
        // from outside, so only the decoded value is pinned.
        rows.push(Row::new(
            key(&format!("vib_type({model:?})")),
            match b.vib_type {
                Some(at) => format!("{at:#04x}[7:5]"),
                None => "—".to_string(),
            },
            None,
            format!("{:?}", o.vib_type(model)),
        ));

        if let Some((e1, e2)) = b.effect {
            for (preset, at) in [(1u8, e1), (2, e2)] {
                rows.push(Row::new(
                    key(&format!("vib_on({model:?},{preset})")),
                    format!("{at:#04x}[3:3]"),
                    u64::from(o.vib_on(model, preset)),
                    o.vib_on(model, preset).to_string(),
                ));
            }
        }
    }

    for preset in [1u8, 2] {
        rows.push(Row::new(
            key(&format!("b3_perc_on({preset})")),
            format!("{:#04x}[2:2]", if preset == 2 { 0x60 } else { 0x59 }),
            u64::from(o.b3_perc_on(preset)),
            o.b3_perc_on(preset).to_string(),
        ));
    }
    rows.push(Row::new(
        key("b3_perc_third"),
        "0x51[4:4]",
        u64::from(o.b3_perc_third()),
        o.b3_perc_third().to_string(),
    ));
    rows.push(Row::new(
        key("b3_perc_speed"),
        "0x51[3:2]",
        None,
        format!("{:?}", o.b3_perc_speed()),
    ));

    // The bass manual's two bars live outside the nibble block — see
    // `OrganPanel::b3_bass_drawbars`.
    rows.push(Row::new(
        key("b3_bass_drawbars"),
        "0x59[3:0]+0x5a[7:0]",
        None,
        format!("{:?}", o.b3_bass_drawbars()),
    ));

    rows
}

/// Every field of every panel of one program, in file order.
pub fn rows(p: &Program) -> Vec<Row> {
    let s = &p.schema;
    let mut rows = packed(&s.center_panel);
    rows.extend(packed(&s.piano_panel));
    rows.extend(packed(&s.sample_panel));
    rows.extend(organ(&s.organ_panel));
    rows.extend(packed(&s.effects_panel));
    rows
}

pub fn read_program(path: &Path) -> Program {
    match nord_format::from_path(path)
        .unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()))
    {
        Entity::Program(nord_format::Program::Electro5(p)) => p as electro5::Program,
        other => panic!("{} is not an Electro 5 program: {other:?}", path.display()),
    }
}

pub fn read_settings(path: &Path) -> electro5::Settings {
    match nord_format::from_path(path)
        .unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()))
    {
        Entity::Settings(nord_format::Settings::Electro5(s)) => s,
        other => panic!("{} is not Electro 5 settings: {other:?}", path.display()),
    }
}

pub fn read_sample(path: &Path) -> nord_format::common::sample::Sample {
    match nord_format::from_path(path)
        .unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()))
    {
        Entity::Sample(s) => s,
        other => panic!("{} is not a sample: {other:?}", path.display()),
    }
}

/// `pending/` is a staging area of untracked local files, not corpus content — a sweep
/// would otherwise take in whatever happened to be sitting in one checkout.
const UNTRACKED: &str = "pending";

/// Every file under `root` with extension `ext`, recursively, in a stable order.
pub fn files_with(root: &Path, ext: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n != UNTRACKED) {
                    stack.push(path);
                }
            } else if path.extension().is_some_and(|e| e == ext) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Every `.ne5p` the corpus ships, in a stable order.
pub fn all_programs(root: &Path) -> Vec<PathBuf> {
    let found = files_with(root, "ne5p");
    assert!(
        found.len() > 100,
        "found only {} programs under {} — is this the ne5 tree of a nord-corpus \
         checkout?",
        found.len(),
        root.display()
    );
    found
}

/// Vendor material, by the corpus's own rule: **a path with a `factory/` component
/// carries no filename oracle**. Its names are Clavia's program names, so a sweep that
/// reads a filename as ground truth has to drop these, while a round-trip sweep wants
/// them.
///
/// Machine-checkable on purpose. An oracle sweep that filters on this cannot be broken
/// by a new model directory arriving, and cannot quietly rot the way a hand-maintained
/// exclusion list does.
pub fn is_factory(path: &Path) -> bool {
    path.components()
        .any(|c| c.as_os_str() == std::ffi::OsStr::new("factory"))
}

/// A specimen the corpus marks as not-yet-explainable, by the `.skip.` in its name.
///
/// The convention has to stay visible: a sweep that quietly `continue`s past one loses
/// the specimen without saying so.
pub fn is_skipped(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|n| n.to_string_lossy().contains(".skip."))
}

/// A specimen's path as a trial names it: relative to the corpus root, so a failure
/// names a file the reader can open.
pub fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}
