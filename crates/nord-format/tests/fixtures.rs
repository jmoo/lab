//! Synthetic specimens, and the suite that runs over them under every feature set.
//!
//! Every byte in `tests/fixtures/` is emitted by this crate's own writers from a
//! constructor plus the edits its filename names. Nothing here came off an instrument or
//! out of the private corpus, which is what lets a public repo ship it and what makes
//! `cargo test` from a fresh clone verify the parse, the round trip and the checksum
//! without a corpus in sight.
//!
//! The filename is the oracle, as in the corpus sweeps: `center-gain-96.ne5p` must
//! decode with `center_panel.gain` at 96.
//!
//! ⚠️ The committed bytes are the golden. [`fixtures_match_their_generator`] fails if a
//! writer change moves them; regenerate with
//!
//! ```sh
//! cargo test -p nord-format --test fixtures -- --ignored bless
//! ```
//!
//! and read the diff before committing it — a fixture that changed because the writer
//! regressed looks exactly like one that changed because the writer improved.

use nord_format::electro5;
use nord_format::{Entity, Live, Program, Settings, Song};
use std::io::Cursor;
use std::path::PathBuf;

/// How a fixture is built, and what its name claims about the result.
enum Spec {
    /// Constructor plus `(field path, value)` edits, applied through `Schema::set_field`
    /// — the same path `nord … edit --set` takes. The extension picks the constructor.
    Fields(&'static [(&'static str, &'static str)]),
    /// A song: its version, then the four program slots it points at as `(bank, slot)`.
    /// Songs carry no settable fields; their four references are the whole body.
    Song(u32, [(u16, u16); 4]),
}

use Spec::{Fields, Song as SongSpec};

/// Every committed fixture. The directory may hold nothing else — see
/// [`the_fixture_directory_holds_exactly_these`].
const FIXTURES: &[(&str, Spec)] = &[
    (
        "center-gain-96.ne5p",
        Fields(&[("center_panel.gain", "96")]),
    ),
    (
        "center-split-C4.ne5p",
        Fields(&[
            ("center_panel.split", "true"),
            ("center_panel.split_point", "C4"),
        ]),
    ),
    ("default.ne5l", Fields(&[])),
    ("default.ne5p", Fields(&[])),
    ("default.ne5s", Fields(&[])),
    ("default.ne5t", SongSpec(0, [(0, 0); 4])),
    (
        "effects-fx1-rate-64.ne5p",
        Fields(&[("effects_panel.fx1_rate", "64")]),
    ),
    ("live-gain-32.ne5l", Fields(&[("center_panel.gain", "32")])),
    (
        "organ-perc-third-on.ne5p",
        Fields(&[("organ_panel.b3_perc_third", "true")]),
    ),
    ("piano-touch-3.ne5p", Fields(&[("piano_panel.touch", "3")])),
    (
        "sample-number-211.ne5p",
        Fields(&[("sample_panel.number", "211")]),
    ),
    (
        "selection-live-2.ne5s",
        Fields(&[
            ("selection.live_mode", "true"),
            ("selection.live_slot", "Live2"),
        ]),
    ),
    (
        "settings-key-click-higher.ne5s",
        Fields(&[("panel.b3_key_click_level", "Higher")]),
    ),
    (
        "settings-transpose-4.ne5s",
        Fields(&[("panel.global_transpose", "4")]),
    ),
    (
        "song-v1-0_1-1_2-2_3-3_4.ne5t",
        SongSpec(1, [(0, 1), (1, 2), (2, 3), (3, 4)]),
    ),
];

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn extension(name: &str) -> &str {
    name.rsplit_once('.')
        .expect("fixture name has an extension")
        .1
}

/// Build a fixture's bytes from its spec — the one definition of what the committed
/// file must contain, shared by the check and by [`bless`].
fn generate(name: &str, spec: &Spec) -> Vec<u8> {
    let entity = match spec {
        SongSpec(version, programs) => {
            let mut song = electro5::Song::new(
                (0, 0).try_into().unwrap(),
                programs.map(|at| at.try_into().unwrap()),
            );
            song.set_version(*version);
            Entity::Song(Song::Electro5(song))
        }
        Fields(edits) => {
            let apply = |set: &mut dyn FnMut(&str, &str) -> Result<(), String>| {
                for (path, value) in *edits {
                    set(path, value).unwrap_or_else(|e| panic!("{name}: setting {path}: {e}"));
                }
            };
            match extension(name) {
                "ne5l" => {
                    let mut live = electro5::Live::new((0, 0).try_into().unwrap());
                    apply(&mut |p, v| live.body.set_field(p, v).map_err(|e| e.to_string()));
                    Entity::Live(Live::Electro5(live))
                }
                "ne5p" => {
                    let mut program = electro5::Program::new((0, 0).try_into().unwrap());
                    apply(&mut |p, v| program.body.set_field(p, v).map_err(|e| e.to_string()));
                    Entity::Program(Program::Electro5(program))
                }
                "ne5s" => {
                    let mut settings = electro5::Settings::new();
                    apply(&mut |p, v| settings.body.set_field(p, v).map_err(|e| e.to_string()));
                    Entity::Settings(Settings::Electro5(settings))
                }
                other => panic!("{name}: no constructor for .{other}"),
            }
        }
    };
    nord_format::to_bytes(&entity).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn committed(name: &str) -> Vec<u8> {
    std::fs::read(dir().join(name)).unwrap_or_else(|e| {
        panic!("{name}: {e} — regenerate with `--ignored bless` (see this file's header)")
    })
}

/// The bytes on disk are the ones the generator emits today.
#[test]
fn fixtures_match_their_generator() {
    for (name, spec) in FIXTURES {
        assert_eq!(
            committed(name),
            generate(name, spec),
            "{name} is not what the generator now emits — a writer changed. Re-bless \
             only once you know which way the change runs.",
        );
    }
}

/// Every fixture parses and re-emits byte for byte.
#[test]
fn every_fixture_round_trips() {
    for (name, _) in FIXTURES {
        let bytes = committed(name);
        let entity = nord_format::from_stream(&mut Cursor::new(&bytes))
            .unwrap_or_else(|e| panic!("{name} failed to parse: {e}"));
        assert_eq!(
            nord_format::to_bytes(&entity).unwrap(),
            bytes,
            "{name} did not re-emit the bytes it came from",
        );
    }
}

/// A body byte flipped under the header's checksum is refused on read.
///
/// The round trip alone cannot show the checksum is verified rather than merely copied
/// — a reader that never compared it would round-trip just as happily.
#[test]
fn every_fixture_refuses_a_corrupted_body() {
    for (name, _) in FIXTURES {
        let mut bytes = committed(name);
        *bytes.last_mut().unwrap() ^= 0xff;
        assert!(
            nord_format::from_stream(&mut Cursor::new(&bytes)).is_err(),
            "{name} parsed with a corrupted body — the checksum is not being checked",
        );
    }
}

/// Each fixture decodes to the edits its filename names.
#[test]
fn every_fixture_decodes_to_its_name() {
    for (name, spec) in FIXTURES {
        let bytes = committed(name);
        let entity = nord_format::from_stream(&mut Cursor::new(&bytes)).unwrap();

        match (spec, &entity) {
            (SongSpec(version, programs), Entity::Song(Song::Electro5(song))) => {
                assert_eq!(song.version(), *version, "{name}: version");
                for (slot, want) in programs.iter().enumerate() {
                    assert_eq!(song.get(slot as u16), *want, "{name}: slot {slot}");
                }
            }
            (Fields(edits), _) => {
                let fields = match &entity {
                    Entity::Live(Live::Electro5(l)) => l.body.fields(),
                    Entity::Program(Program::Electro5(p)) => p.body.fields(),
                    Entity::Settings(Settings::Electro5(s)) => s.body.fields(),
                    other => panic!("{name} decoded as {other:?}"),
                };
                for (path, value) in *edits {
                    let field = fields
                        .iter()
                        .find(|f| f.path == *path)
                        .unwrap_or_else(|| panic!("{name}: no field {path}"));
                    assert_eq!(&field.value, value, "{name}: {path}");
                }
            }
            (spec, entity) => panic!(
                "{name}: a {} spec built a {entity:?}",
                match spec {
                    Fields(_) => "field",
                    SongSpec(..) => "song",
                }
            ),
        }
    }
}

/// No fixture sits in the directory unchecked, and none the table names is missing.
#[test]
fn the_fixture_directory_holds_exactly_these() {
    let mut found: Vec<String> = std::fs::read_dir(dir())
        .expect("tests/fixtures")
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();

    let mut listed: Vec<String> = FIXTURES.iter().map(|(n, _)| n.to_string()).collect();
    listed.sort();

    assert_eq!(found, listed, "tests/fixtures disagrees with the table");
}

/// Rewrite every fixture from its spec. Run deliberately; see this file's header.
#[test]
#[ignore = "writes tests/fixtures; run it when a writer change is meant to move them"]
fn bless() {
    std::fs::create_dir_all(dir()).unwrap();
    for (name, spec) in FIXTURES {
        std::fs::write(dir().join(name), generate(name, spec)).unwrap();
    }
}
