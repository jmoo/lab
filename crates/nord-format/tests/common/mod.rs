//! What every corpus test target needs: the corpus root, one program's decode as
//! comparable rows, and the oracle sidecars.
//!
//! ⚠️ A rustc-visible module, not a test target — each corpus test compiles its own copy,
//! so an item no single target uses is still live.
#![allow(dead_code)]

use nord_format::electro5::program::OrganPanel;
use nord_format::electro5::{OrganModel, Program};
use nord_format::panel::Panel;
use nord_format::{electro5, Entity};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Root of the specimen corpus, taken from `NORD_CORPUS_DIR` — **the whole corpus**,
/// the directory holding `ne5/`, `ne6/`, … and the sample-pool directories `nsmp/`,
/// `nsmp3/`, `nsmp4/`, not one model.
///
/// Since these tests only compile under the `corpus` feature, a missing
/// `NORD_CORPUS_DIR` is a hard error, not a skip.
pub fn corpus_dir() -> PathBuf {
    std::env::var_os("NORD_CORPUS_DIR")
        .expect("set NORD_CORPUS_DIR to a nord-corpus checkout root for --features corpus")
        .into()
}

/// The Electro 5 model directory.
///
/// Every specimen the corpus can oracle is Electro 5, so the decode suite joins this
/// rather than treating the corpus root as a model root. A suite that walks every model
/// takes [`corpus_dir`] instead.
pub fn ne5_dir() -> PathBuf {
    corpus_dir().join("ne5")
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
    let s = &p.body;
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

// ---------------------------------------------------------------------------
// The oracle sidecars
// ---------------------------------------------------------------------------

/// The suffix a specimen's oracle sidecar carries: `<specimen filename>.oracle.json`.
///
/// ⚠️ It must not end in an extension any corpus walker sweeps on. The walkers key on
/// `Path::extension`, which sees `json` here, so a sidecar is never mistaken for a
/// specimen.
pub const SIDECAR: &str = ".oracle.json";

/// The reserved sidecar name for facts about a whole directory rather than one file.
pub const DIR_SIDECAR: &str = "dir.oracle.json";

/// Where a specimen's sidecar sits.
pub fn sidecar_of(specimen: &Path) -> PathBuf {
    let mut name = specimen.file_name().unwrap().to_os_string();
    name.push(SIDECAR);
    specimen.with_file_name(name)
}

/// The specimen a sidecar belongs to, or `None` for the directory sidecar.
pub fn specimen_of(sidecar: &Path) -> Option<PathBuf> {
    let name = sidecar.file_name()?.to_string_lossy().into_owned();
    if name == DIR_SIDECAR {
        return None;
    }
    Some(sidecar.with_file_name(name.strip_suffix(SIDECAR)?))
}

/// Every `*.oracle.json` under `root`, in a stable order. Includes directory sidecars.
pub fn sidecars(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if path.is_dir() {
                if name != UNTRACKED && !name.starts_with('.') {
                    stack.push(path);
                }
            } else if name.ends_with(SIDECAR) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// One pinned value: what the capture says the field holds, and how far the decode may
/// sit from it.
pub struct Expect {
    pub value: String,
    /// `None` for an exact match. Otherwise both sides are read as numbers and must be
    /// within this of each other — the analog knobs, whose capture is a position the
    /// operator aimed at rather than the one the shaft landed on.
    pub slack: Option<f64>,
}

/// A specimen's oracle, as its sidecar spells it.
pub struct Oracle {
    /// Field path -> what it must hold. Empty when the specimen is deliberately
    /// unoracled.
    pub fields: BTreeMap<String, Expect>,
    /// Why this specimen pins nothing. Mutually exclusive with `fields`.
    pub unoracled: Option<String>,
    /// A sibling filename whose bytes this specimen's must equal — the relational
    /// oracle, for a capture that was made to move something and moved nothing.
    pub same_body_as: Option<String>,
    /// Named mechanical properties the checkers understand. See the corpus README for
    /// the vocabulary.
    pub traits: Vec<String>,
}

/// Read one sidecar. A malformed one is a hard error: a sidecar the reader silently
/// skips is an oracle that has stopped asserting anything.
pub fn read_oracle(path: &Path) -> Oracle {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let json: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let at = |what: &str| format!("{}: {what}", path.display());

    assert_eq!(
        json["schema"].as_u64(),
        Some(1),
        "{}",
        at("schema must be 1")
    );

    let mut fields = BTreeMap::new();
    if let Some(map) = json.get("fields") {
        for (key, want) in map
            .as_object()
            .unwrap_or_else(|| panic!("{}", at("`fields` is not an object")))
        {
            let expect = match want {
                serde_json::Value::String(s) => Expect {
                    value: s.clone(),
                    slack: None,
                },
                serde_json::Value::Object(o) => Expect {
                    value: o["value"]
                        .as_str()
                        .unwrap_or_else(|| panic!("{}", at(&format!("{key}: no `value`"))))
                        .to_string(),
                    slack: o.get("slack").map(|s| {
                        s.as_f64()
                            .unwrap_or_else(|| panic!("{}", at(&format!("{key}: bad `slack`"))))
                    }),
                },
                _ => panic!("{}", at(&format!("{key}: not a string or an object"))),
            };
            fields.insert(key.clone(), expect);
        }
    }

    let unoracled = json.get("unoracled").map(|v| {
        v.as_str()
            .expect("`unoracled` is a reason string")
            .to_string()
    });
    assert!(
        unoracled.is_none() || fields.is_empty(),
        "{}",
        at("an unoracled specimen cannot also pin fields"),
    );

    Oracle {
        fields,
        unoracled,
        same_body_as: json.get("same_body_as").map(|v| {
            v.as_str()
                .expect("`same_body_as` is a filename")
                .to_string()
        }),
        traits: json
            .get("traits")
            .map(|v| {
                v.as_array()
                    .expect("`traits` is an array")
                    .iter()
                    .map(|t| t.as_str().expect("a trait is a string").to_string())
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// Every value of one specimen an oracle may address, under the path it addresses it by.
///
/// Each path maps to the value's two canonical spellings: the way `nord inspect` prints
/// it, and the way `--set` takes it. They differ only for a field too wide to have named
/// values — the drawbar blocks and the dependency ids — where the first is a list or a
/// decimal and the second the stored bits. A sidecar may use either.
///
/// Beyond the panel fields the view carries a few derived paths, for state that is real
/// but is not one placement: the entity's slot, the organ accessors (which model reads
/// which block is not expressible as a bit range), the two halves of the part mix, and a
/// sample's name and zone layout.
pub fn oracle_view(path: &Path) -> BTreeMap<String, [String; 2]> {
    let mut view: BTreeMap<String, [String; 2]> = BTreeMap::new();
    let mut put = |key: String, display: String| {
        let both = [display.clone(), display];
        view.insert(key, both);
    };

    match nord_format::from_path(path)
        .unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()))
    {
        Entity::Program(nord_format::Program::Electro5(p)) => {
            put("location".into(), format!("{:?}", p.location.inner()));
            program_view(&p.body, &mut view);
        }
        Entity::Live(nord_format::Live::Electro5(l)) => {
            put("location".into(), format!("{:?}", l.location.inner()));
            program_view(&l.body, &mut view);
        }
        Entity::Settings(nord_format::Settings::Electro5(s)) => {
            for f in s.body.fields() {
                view.insert(f.path, [f.display, f.value]);
            }
        }
        Entity::Song(nord_format::Song::Electro5(s)) => {
            put("location".into(), format!("{:?}", s.location.inner()));
            let programs: Vec<(u16, u16)> = (0..4).map(|i| s.get(i).inner()).collect();
            put("programs".into(), format!("{programs:?}"));
        }
        Entity::Sample(s) => {
            put("name".into(), s.name().unwrap());
            put("version".into(), s.header.version.to_string());
            let roots: Vec<u8> = s.strokes().unwrap().iter().map(|k| k.root_key).collect();
            let tops: Vec<u8> = s.zones().unwrap().iter().map(|z| z.top_note).collect();
            put("root_keys".into(), format!("{roots:?}"));
            put("top_notes".into(), format!("{tops:?}"));
        }
        other => panic!("{} has no oracle view: {other:?}", path.display()),
    }
    view
}

fn program_view(schema: &electro5::program::Schema, view: &mut BTreeMap<String, [String; 2]>) {
    for f in schema.fields() {
        view.insert(f.path, [f.display, f.value]);
    }
    let mix = &schema.center_panel.part_mix;
    for (half, value) in [("lower", mix.lower()), ("upper", mix.upper())] {
        let text = value.to_string();
        view.insert(
            format!("center_panel.part_mix.{half}"),
            [text.clone(), text],
        );
    }
    // The organ accessors under the panel's own prefix. `organ` is the hand-written
    // second statement of where the organ's state lives — see its doc — so the oracle
    // reaches the meaning of a model's registration, not only its storage.
    for row in organ(&schema.organ_panel) {
        let key = row.key.replace("OrganPanel.", "organ_panel.");
        view.insert(key, [row.value.clone(), row.value]);
    }
}

/// Compare one pinned value against the decode. `slack` reads both sides as numbers.
pub fn check_field(view: &BTreeMap<String, [String; 2]>, path: &str, want: &Expect, at: &str) {
    let got = view
        .get(path)
        .unwrap_or_else(|| panic!("{at}: nothing at oracle path {path}"));
    match want.slack {
        None => assert!(
            got.contains(&want.value),
            "{at}: {path} is {:?}, the oracle says {:?}",
            got[0],
            want.value,
        ),
        Some(slack) => {
            let number = |s: &str| {
                s.parse::<f64>()
                    .unwrap_or_else(|_| panic!("{at}: {path}: {s:?} is not a number"))
            };
            let (got, want_n) = (number(&got[0]), number(&want.value));
            assert!(
                (got - want_n).abs() <= slack,
                "{at}: {path} is {got}, the oracle says {want_n} (± {slack})",
            );
        }
    }
}

/// A sidecar's raw JSON, for the blocks [`read_oracle`] does not model — the golden
/// tables a directory sidecar carries.
pub fn read_json(path: &Path) -> serde_json::Value {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}
