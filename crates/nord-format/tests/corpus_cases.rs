//! The corpus, one reported case per specimen.
//!
//! A `libtest-mimic` harness rather than `#[test]` sweeps: every specimen is its own
//! named trial, so a failure names the file, one failure no longer hides the rest of the
//! directory, and a `.skip.` specimen is an **ignored** case in the report instead of a
//! silent `continue`.
//!
//! Trials are named `<check>/<path under the corpus root>`, so the usual filter is a
//! path fragment:
//!
//! ```sh
//! NORD_CORPUS_DIR=/path/to/nord-corpus \
//!   cargo test -p nord-format --features corpus --test corpus_cases -- programs/organ
//! ```
//!
//! The checks:
//!
//! * `round_trip` — the file re-emits byte for byte, for every format the corpus ships.
//! * `decode` — the **oracle sidecar**. Every specimen the corpus can say something
//!   about ships a `<filename>.oracle.json` beside it, pinning the fields its capture
//!   fixed; this walks them. **This is the real decode test**: a round trip passes
//!   whatever the fields read, because a panel keeps the bytes it came from.
//! * `mutation` — assigning to a field reaches the bytes, reads back, and moves nothing
//!   outside that panel's span.
//! * `drawbars` — reading nine drawbar nibbles and writing them back is a no-op.
//! * `named_values` — no specimen decodes to a value no component can name.
//! * `as_program` — a live body retagged as a program decodes identically.
//! * `strokes` / `zone_ranges` — the `.nsmp` section chain and its key ranges.
//! * `corpus_wide` — the assertions that need every program at once.
//! * `coverage` — the floors, and the standing check that an absent oracle is a decision
//!   rather than an omission.
//!
//! The bit-flip harness is `mutation.rs` and the per-field pins are
//! `decode_snapshot.rs`.

#[cfg(feature = "corpus")]
mod common;

#[cfg(feature = "corpus")]
mod cases {
    use super::common::{
        all_programs, check_field, files_with, is_factory, is_skipped, ne5_dir, oracle_view,
        read_json, read_oracle, read_program, read_sample, read_settings, rel, sidecar_of,
        sidecars, specimen_of, Oracle, DIR_SIDECAR,
    };
    use libtest_mimic::{Arguments, Trial};
    use nord_format::common::bank::Item;
    use nord_format::common::container;
    use nord_format::electro5::{EqualizerPart, PianoCategory};
    use nord_format::{electro5, Entity};
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs::read;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};

    // -----------------------------------------------------------------------
    // The harness
    // -----------------------------------------------------------------------

    pub fn run() -> ! {
        let args = Arguments::from_args();
        let root = ne5_dir();
        let mut trials = Vec::new();

        let programs = all_programs(&root);
        // The panel sweeps take differential material only: `is_factory` is the
        // corpus-wide rule that keeps vendor content — whose names are program names,
        // not settings — out of a check that is about one knob at a time.
        let sweep: Vec<PathBuf> = programs
            .iter()
            .filter(|p| p.starts_with(root.join("programs")) && !is_factory(p))
            .cloned()
            .collect();

        // Every format the corpus ships re-emits through one code path, so the round
        // trip is one check over all of them rather than one per format.
        for ext in ["ne5p", "ne5s", "ne5t", "ne5l", "nsmp"] {
            for path in files_with(&root, ext) {
                trials.push(trial("round_trip", &root, path, round_trip));
            }
        }

        // The decode sweep is the sidecars, wherever they sit. A specimen joins it by
        // gaining an oracle in the corpus, not by anyone adding a directory here.
        for sidecar in sidecars(&root) {
            if let Some(specimen) = specimen_of(&sidecar) {
                trials.push(trial("decode", &root, specimen, decode));
            }
        }

        for path in &sweep {
            trials.push(trial("mutation", &root, path.clone(), mutation));
            trials.push(trial("drawbars", &root, path.clone(), drawbars));
        }
        for path in &programs {
            trials.push(trial("named_values", &root, path.clone(), program_values));
        }
        for path in files_with(&root, "ne5s") {
            trials.push(trial("named_values", &root, path, settings_values));
        }

        for path in files_with(&root, "ne5l") {
            trials.push(trial("as_program", &root, path, live_decodes_as_program));
        }

        for path in files_with(&root.join("samples"), "nsmp") {
            trials.push(trial("strokes", &root, path.clone(), strokes));
            trials.push(trial("zone_ranges", &root, path, zone_ranges));
        }

        trials.extend(corpus_wide(&root));
        trials.extend(coverage(&root));

        libtest_mimic::run(&args, trials).exit()
    }

    /// One specimen, one check. A `.skip.` specimen becomes a reported ignored case:
    /// the convention stays visible instead of being a `continue` nobody counts.
    fn trial(kind: &str, root: &Path, path: PathBuf, check: fn(&Path)) -> Trial {
        let ignored = is_skipped(&path);
        Trial::test(format!("{kind}/{}", rel(root, &path)), move || {
            check(&path);
            Ok(())
        })
        .with_ignored_flag(ignored)
    }

    /// A named trial that is not about one specimen — a floor, or an assertion whose
    /// subject is the whole corpus.
    fn named(prefix: &str, name: &str, check: impl FnOnce() + Send + 'static) -> Trial {
        Trial::test(format!("{prefix}/{name}"), move || {
            check();
            Ok(())
        })
    }

    fn stem(path: &Path) -> String {
        path.file_stem().unwrap().to_string_lossy().into_owned()
    }

    fn name_of(path: &Path) -> String {
        path.file_name().unwrap().to_string_lossy().into_owned()
    }

    // -----------------------------------------------------------------------
    // round_trip
    // -----------------------------------------------------------------------

    /// Re-encoding reproduces the file byte for byte, including the bits no field claims.
    fn round_trip(path: &Path) {
        let original = read(path).unwrap();
        let entity = nord_format::from_path(path)
            .unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()));
        let rewritten = nord_format::to_bytes(&entity).unwrap();
        assert_eq!(
            original.as_slice(),
            rewritten.as_slice(),
            "re-encoding changed {}",
            path.display(),
        );
    }

    // -----------------------------------------------------------------------
    // decode — the oracle sidecars
    // -----------------------------------------------------------------------

    /// The specimen decodes to what its sidecar says it holds.
    ///
    /// The sidecar is the machine-readable half of the filename: the corpus writes down,
    /// once, what a capture fixed, and this compares it against the decode by field path.
    /// Nothing here knows what a panel or a knob is — the oracle names the field and the
    /// value, and a specimen the corpus cannot say anything about says *that*, so an
    /// absent oracle stays a decision.
    fn decode(path: &Path) {
        let at = path.display().to_string();
        let oracle = read_oracle(&sidecar_of(path));

        if let Some(why) = &oracle.unoracled {
            println!("deliberately unoracled: {why}");
        } else {
            assert!(
                !oracle.fields.is_empty() || oracle.same_body_as.is_some(),
                "{at}: the sidecar pins nothing and does not say why",
            );
            let view = oracle_view(path);
            for (key, want) in &oracle.fields {
                check_field(&view, key, want, &at);
            }
        }

        if let Some(sibling) = &oracle.same_body_as {
            assert_eq!(
                read(path).unwrap(),
                read(path.with_file_name(sibling)).unwrap(),
                "{at} is no longer byte-identical to {sibling} — the corpus gained a \
                 capture that moves something, so this specimen can now be asserted",
            );
        }

        for property in &oracle.traits {
            check_trait(path, property, &oracle);
        }
    }

    /// The mechanical properties a sidecar can claim, which are checks rather than
    /// values. See the corpus README for the vocabulary.
    fn check_trait(path: &Path, property: &str, oracle: &Oracle) {
        let at = path.display().to_string();
        match property {
            // b3+bass preset 1 keeps its two bass drawbars outside the nine-nibble
            // block, and the two nibbles they shadow there hold stale leftovers. The
            // sidecar pins the bass pair; this is the other half — that the pair is
            // genuinely being read from elsewhere.
            "b3_bass_manual" => {
                use nord_format::electro5::OrganModel;
                let organ = read_program(path).schema.organ_panel;
                let bass = organ.b3_bass_drawbars();
                let main = organ.drawbars(OrganModel::B3, 1);
                assert!(
                    bass == [0, 0] || (main[0], main[1]) != (bass[0], bass[1]),
                    "{at}: the bass values also appear in the main block — offsets may \
                     be wrong",
                );
            }
            // Consumed by `zone_ranges`, which is a sweep of its own.
            "zone_top_notes_overridden" => {
                assert!(
                    oracle.fields.contains_key("top_notes"),
                    "{at}: a hand-edited zone layout has to state what it is",
                );
            }
            other => panic!("{at}: no checker knows the trait {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // mutation / drawbars / named_values
    // -----------------------------------------------------------------------

    type MutationCase = (
        &'static str,
        std::ops::RangeInclusive<usize>,
        fn(&mut electro5::Program),
        fn(&electro5::Program),
    );

    fn mutation_cases() -> Vec<MutationCase> {
        vec![
            (
                "center_panel.gain",
                0x2e..=0x34,
                |p| p.schema.center_panel.gain = 96u8.try_into().unwrap(),
                |p| assert_eq!(p.schema.center_panel.gain, 96),
            ),
            (
                "piano_panel.touch",
                0x3a..=0x41,
                |p| p.schema.piano_panel.touch = 3u8.try_into().unwrap(),
                |p| assert_eq!(p.schema.piano_panel.touch, 3),
            ),
            (
                "sample_panel.number",
                0x46..=0x4d,
                |p| p.schema.sample_panel.number = 211,
                |p| assert_eq!(p.schema.sample_panel.number, 211),
            ),
            (
                "organ_panel.b3_perc_third",
                0x4e..=0x92,
                |p| p.schema.organ_panel.set_b3_perc_third(true),
                |p| assert!(p.schema.organ_panel.b3_perc_third()),
            ),
            (
                "effects_panel.fx3_compression",
                0x93..=0x9f,
                |p| p.schema.effects_panel.fx3_compression = 101u8.try_into().unwrap(),
                |p| assert_eq!(p.schema.effects_panel.fx3_compression, 101),
            ),
            (
                // Three bits in 0x9a, four in 0x9b.
                "effects_panel.equalizer_freq_gain",
                0x93..=0x9f,
                |p| p.schema.effects_panel.equalizer_freq_gain = 0x55u8.try_into().unwrap(),
                |p| assert_eq!(p.schema.effects_panel.equalizer_freq_gain, 0x55),
            ),
            (
                // Five bits in 0x9e, two in 0x9f.
                "effects_panel.fx5_moisture",
                0x93..=0x9f,
                |p| p.schema.effects_panel.fx5_moisture = 0x2au8.try_into().unwrap(),
                |p| assert_eq!(p.schema.effects_panel.fx5_moisture, 0x2a),
            ),
            (
                "effects_panel.equalizer_part",
                0xa1..=0xa4,
                |p| p.schema.effects_panel.equalizer_part = EqualizerPart::Both,
                |p| assert_eq!(p.schema.effects_panel.equalizer_part, EqualizerPart::Both),
            ),
        ]
    }

    /// Assigning to a field reaches the bytes, reads back, and moves nothing outside that
    /// panel's own byte span.
    fn mutation(path: &Path) {
        let name = path.display().to_string();
        let original = read(path).unwrap();

        for (field, span, mutate, check) in mutation_cases() {
            let mut program = read_program(path);
            mutate(&mut program);

            let mut mutated: Vec<u8> = Vec::new();
            program.write_to(&mut Cursor::new(&mut mutated)).unwrap();

            for (at, (before, after)) in original.iter().zip(&mutated).enumerate() {
                // 0x18..=0x1b is the body checksum, which any body change moves.
                let allowed = span.contains(&at) || (0x18..=0x1b).contains(&at);
                assert!(
                    allowed || before == after,
                    "setting {field} changed byte {at:#04x} of {name}",
                );
            }

            let Entity::Program(nord_format::Program::Electro5(reread)) =
                nord_format::from_stream(&mut Cursor::new(&mut mutated)).unwrap()
            else {
                panic!("expected an electro5 program in {name}")
            };
            check(&reread);
        }
    }

    /// Reading nine drawbar nibbles and writing them back is a no-op.
    fn drawbars(path: &Path) {
        use electro5::OrganModel::*;

        let original = read(path).unwrap();
        let mut program = read_program(path);

        let organ = &mut program.schema.organ_panel;
        for model in [B3, Vox, Farfisa, Pipe] {
            for preset in [1u8, 2] {
                let bars = organ.drawbars(model, preset);
                if bars.iter().any(|&b| b > 8) {
                    continue;
                }
                organ.set_drawbars(model, preset, bars).unwrap();
            }
        }

        let mut rewritten: Vec<u8> = Vec::new();
        program.write_to(&mut Cursor::new(&mut rewritten)).unwrap();
        assert_eq!(
            original.as_slice(),
            rewritten.as_slice(),
            "rewriting the drawbars changed {}",
            path.display(),
        );
    }

    /// No specimen decodes to an `Unknown` in a sparse component.
    ///
    /// The decoder preserves unrecognized values rather than refusing them, so this is
    /// where they get noticed. A failure means a specimen produced a value with no name
    /// yet — go name it.
    fn program_values(path: &Path) {
        let p = read_program(path);
        let (c, pi, fx) = (
            &p.schema.center_panel,
            &p.schema.piano_panel,
            &p.schema.effects_panel,
        );

        let mut unknowns: BTreeMap<&'static str, u64> = BTreeMap::new();
        let mut check = |field: &'static str, unknown: bool, raw: u64| {
            if unknown {
                unknowns.insert(field, raw);
            }
        };
        check(
            "center.organ_type",
            c.organ_type.is_unknown(),
            c.organ_type.raw().into(),
        );
        check(
            "piano.category",
            pi.category.is_unknown(),
            pi.category.raw().into(),
        );
        check(
            "fx.fx1_type",
            fx.fx1_type.is_unknown(),
            fx.fx1_type.raw().into(),
        );
        check(
            "fx.fx2_type",
            fx.fx2_type.is_unknown(),
            fx.fx2_type.raw().into(),
        );
        check(
            "fx.fx3_type",
            fx.fx3_type.is_unknown(),
            fx.fx3_type.raw().into(),
        );
        check(
            "fx.fx5_type",
            fx.fx5_type.is_unknown(),
            fx.fx5_type.raw().into(),
        );
        check(
            "effects_panel.equalizer_part",
            fx.equalizer_part.is_unknown(),
            fx.equalizer_part.raw().into(),
        );

        assert!(
            unknowns.is_empty(),
            "{} holds values no component can name — worth investigating, not \
             suppressing: {unknowns:?}",
            path.display(),
        );
    }

    /// No settings file decodes to a value with no name.
    ///
    /// Every enumeration here is wider than the values named for it, so an unrecognized
    /// one is preserved rather than refused — this is where it gets noticed.
    fn settings_values(path: &Path) {
        use nord_format::panel::Panel;

        let schema = read_settings(path).schema;
        // Both panels over the body — the selection's `live_slot` can hold an
        // unrecognized value too.
        let unknowns: BTreeMap<String, u64> = schema
            .panel
            .field_values()
            .into_iter()
            .chain(schema.selection.field_values())
            .filter(|v| v.value.starts_with("unknown"))
            .map(|v| (v.name.to_string(), v.raw))
            .collect();

        assert!(
            unknowns.is_empty(),
            "{} holds settings values no component can name — worth investigating, not \
             suppressing: {unknowns:?}",
            path.display(),
        );
    }

    /// A live specimen with its tag swapped is a valid program, down to the last field.
    ///
    /// Confirmed on hardware: the same panel state read as a live slot and as a program
    /// gives byte-identical bodies. One `Schema` serves both, so the field values cannot
    /// disagree — what this pins on real specimens is that everything around them agrees
    /// too: the slot falls in the program space, the version is one programs accept, and
    /// the body checksum still holds.
    fn live_decodes_as_program(path: &Path) {
        let name = path.display().to_string();
        let Entity::Live(nord_format::Live::Electro5(live)) = nord_format::from_path(path).unwrap()
        else {
            panic!("expected an electro5 live slot in {name}")
        };

        // The tag is the whole difference. Retagged through the container, so the file
        // is re-checksummed in whichever generation it came in:
        // ⚠️ a type-1 crc32 covers `0x2c..` and never sees the tag, but a type-0 crc16
        // covers the whole file, header included, so retagging one in place corrupts it.
        let mut file = container::Container::parse(&read(path).unwrap()).unwrap();
        file.header.tag = electro5::program::FORMAT.to_string();
        let bytes = file.to_bytes().unwrap();

        let Entity::Program(nord_format::Program::Electro5(program)) =
            nord_format::from_stream(&mut Cursor::new(&bytes)).unwrap()
        else {
            panic!("a retagged live slot did not decode as a program: {name}")
        };

        let named = |fields: Vec<electro5::program::Field>| -> Vec<(String, String)> {
            fields.into_iter().map(|f| (f.path, f.display)).collect()
        };
        let fields = named(live.schema.fields());
        assert!(!fields.is_empty(), "no fields to compare");
        assert_eq!(
            fields,
            named(program.schema.fields()),
            "live and program decodes disagree on {name}",
        );
    }

    // -----------------------------------------------------------------------
    // Samples (`.nsmp`)
    // -----------------------------------------------------------------------

    /// Every stroke decomposes into its header plus whole packets.
    ///
    /// A wrong header rule shows up here as a leftover remainder.
    fn strokes(path: &Path) {
        let name = stem(path);
        let sample = read_sample(path);
        let zones = sample.zones().unwrap();
        let strokes = sample.strokes().unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            strokes.len(),
            zones.len(),
            "{name}: {} zones but {} strokes",
            zones.len(),
            strokes.len()
        );
    }

    /// Zone key ranges match what the editor lays out from the root keys — **except where
    /// they were overridden by hand**, which the specimen's sidecar says outright.
    ///
    /// So the derivation is the editor's default, not a rule the format enforces: the top
    /// note really is stored, and a reader must take it as read rather than recompute it.
    fn zone_ranges(path: &Path) {
        use nord_format::common::sample::zone;

        let name = stem(path);
        let sample = read_sample(path);
        let roots: Vec<u8> = sample
            .strokes()
            .unwrap()
            .iter()
            .map(|s| s.root_key)
            .collect();
        let stored: Vec<u8> = sample.zones().unwrap().iter().map(|z| z.top_note).collect();
        let derived = zone::derive_top_notes(&roots);

        let sidecar = sidecar_of(path);
        let overridden = sidecar.exists()
            && read_oracle(&sidecar)
                .traits
                .iter()
                .any(|t| t == "zone_top_notes_overridden");

        if overridden {
            assert_ne!(
                stored, derived,
                "{name} is marked hand-edited but matches the default layout"
            );
            return;
        }
        assert_eq!(
            stored, derived,
            "{name}: stored key ranges disagree with the ones its root keys {roots:?} imply"
        );
    }

    // -----------------------------------------------------------------------
    // corpus_wide — the assertions that need every program at once
    // -----------------------------------------------------------------------

    fn corpus_wide(root: &Path) -> Vec<Trial> {
        let mut trials = Vec::new();

        let backup = root.join("usb/backup/full_backup");
        trials.push(named("corpus_wide", "dependency_ids", move || {
            backup_dependency_ids(&backup)
        }));

        let at = root.to_path_buf();
        trials.push(named("corpus_wide", "live_slots", move || {
            // The three live slots are `0:0`, `0:1` and `0:2` on the wire, and nothing
            // else.
            let mut seen: BTreeSet<(u16, u16)> = BTreeSet::new();
            for path in files_with(&at, "ne5l") {
                let name = path.display().to_string();
                let Entity::Live(nord_format::Live::Electro5(live)) =
                    nord_format::from_path(&path).unwrap()
                else {
                    panic!("expected an electro5 live slot in {name}")
                };
                let slot = live.location();
                assert_eq!(slot.x(), 0, "live slot outside bank 0 in {name}");
                seen.insert(slot.inner());
            }
            assert_eq!(
                seen,
                BTreeSet::from([(0, 0), (0, 1), (0, 2)]),
                "the corpus no longer covers all three live slots",
            );
        }));

        trials
    }

    /// Both panels carry a 32-bit reference to the piano (`.npno`) or sample (`.nsmp`)
    /// the program needs. Everything else in those panels is a *slot coordinate* — the
    /// piano category + model dials, the Samp Lib number — which moves when the
    /// instrument's library changes. The id is the stable key: it is what resolves the
    /// song -> program -> piano chain, and what Nord Sound Manager checks before offering
    /// a Restore.
    ///
    /// Each id's *value* is pinned per specimen by the sidecars. What needs every program
    /// at once is the shape: across the backup each id must occupy exactly one (category,
    /// model) slot and vice versa — a too-narrow id over-splits a slot into several ids,
    /// a too-wide one merges distinct pianos under one id — and the model slots the
    /// programs reference must be exactly the ones the backup shipped.
    fn backup_dependency_ids(backup: &Path) {
        // The backup's member list: what the instrument actually shipped. The blobs
        // themselves are R2-tier and absent here, but the listing tells us how many
        // pianos each category held.
        let members = std::fs::read_to_string(backup.join("backup.members.tsv")).unwrap();
        let mut shipped: BTreeMap<String, usize> = BTreeMap::new();
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
                *shipped.entry(category.to_string()).or_default() += 1;
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

        // Per category: the model slots the programs reference must be exactly `0..n`,
        // and `n` must be what the backup shipped — with one exception. The programs
        // reference a seventh Upright that the instrument no longer holds, and that
        // single dangling reference is the whole reason this field matters: it is what
        // Nord Sound Manager sees as a missing dependency and gates "Restore" on. If this
        // trips, check whether the corpus gained or lost a piano before assuming the
        // decode moved.
        //
        // The stored category and the export directory name are not in step — the dial
        // order on disk starts at Grand — so the backup's own sidecar carries the map.
        let map = read_json(&backup.join(DIR_SIDECAR));
        let categories = map["piano_categories"]
            .as_array()
            .expect("the backup sidecar carries `piano_categories`");
        assert_eq!(categories.len(), 6, "a piano category went missing");

        for row in categories {
            let (spelled, directory) = (
                row["category"].as_str().unwrap(),
                row["directory"].as_str().unwrap(),
            );
            let models: BTreeSet<u8> = id_of
                .keys()
                .filter(|(c, _)| format!("{c:?}") == spelled)
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
        // Samp Lib position that the corpus reuses across ids — so bound the id count by
        // the shipped library instead.
        assert!(
            sample_ids.len() <= samples,
            "{} distinct sample ids referenced but only {samples} `.nsmp` members shipped",
            sample_ids.len(),
        );
    }

    // -----------------------------------------------------------------------
    // coverage — the floors, and the oracle's own bookkeeping
    // -----------------------------------------------------------------------

    /// `(trial name, directory under the corpus root, extension, floor)`.
    ///
    /// An empty directory must not read as a pass, which is the failure mode a
    /// per-specimen harness would otherwise introduce: no specimens means no trials means
    /// green. An empty directory means the whole corpus root — songs and live slots ship
    /// inside the full backup rather than in a directory of their own.
    const FLOORS: &[(&str, &str, &str, usize)] = &[
        ("live_slots", "", "ne5l", 3),
        ("programs/center_panel", "programs/center_panel", "ne5p", 1),
        ("programs/equalizer", "programs/equalizer", "ne5p", 1),
        ("programs/fx", "programs/fx", "ne5p", 1),
        ("programs/gain", "programs/gain", "ne5p", 1),
        ("programs/organ", "programs/organ", "ne5p", 1),
        ("programs/piano", "programs/piano", "ne5p", 1),
        ("programs/sample", "programs/sample", "ne5p", 1),
        ("samples", "samples", "nsmp", 30),
        ("settings", "settings", "ne5s", 100),
        ("songs", "", "ne5t", 60),
    ];

    /// `(directory under the corpus root, the extension its specimens carry)` — where
    /// every specimen is expected to answer the oracle, one way or the other.
    ///
    /// These are the differential directories: each file in them was captured to fix
    /// something, so either it says what, or it says why it cannot. The directory's own
    /// `dir.oracle.json` may answer for the lot with a blanket `unoracled`.
    const ORACLE_SWEEP: &[(&str, &str)] = &[
        ("programs/center_panel", "ne5p"),
        ("programs/equalizer", "ne5p"),
        ("programs/fx", "ne5p"),
        ("programs/gain", "ne5p"),
        ("programs/organ", "ne5p"),
        ("programs/piano", "ne5p"),
        ("programs/sample", "ne5p"),
        ("samples", "nsmp"),
        ("settings", "ne5s"),
    ];

    fn coverage(root: &Path) -> Vec<Trial> {
        let mut trials = Vec::new();

        for &(name, dir, ext, floor) in FLOORS {
            let at = root.join(dir);
            trials.push(named("coverage", name, move || {
                let found = files_with(&at, ext).len();
                assert!(
                    found >= floor,
                    "{} holds {found} .{ext} specimens, expected at least {floor} — is \
                     the corpus present?",
                    at.display(),
                );
            }));
        }

        let at = root.to_path_buf();
        trials.push(named("coverage", "oracles_are_a_decision", move || {
            oracles_are_a_decision(&at)
        }));

        // The bass manual is preset 1 of b3+bass only, and it is the one organ field with
        // no home in the nine-nibble block, so its specimen count is worth its own floor.
        let organ = root.join("programs/organ");
        trials.push(named("coverage", "programs/organ/b3_bass", move || {
            let found = files_with(&organ, "ne5p")
                .iter()
                .filter(|p| {
                    let sidecar = sidecar_of(p);
                    sidecar.exists()
                        && read_oracle(&sidecar)
                            .traits
                            .iter()
                            .any(|t| t == "b3_bass_manual")
                })
                .count();
            assert!(
                found >= 4,
                "expected several b3+bass preset-1 specimens, saw {found}"
            );
        }));

        for dir in ["programs/piano", "programs/sample"] {
            let at = root.join(dir);
            trials.push(named(
                "coverage",
                &format!("{dir}/dependencies"),
                move || dependencies(&at),
            ));
        }

        let samples = root.join("samples");
        trials.push(named("coverage", "nsmp_strokes", move || {
            let walked: usize = files_with(&samples, "nsmp")
                .iter()
                .map(|p| read_sample(p).strokes().unwrap().len())
                .sum();
            assert!(
                walked >= 30,
                "only {walked} strokes walked; is the corpus stale?"
            );
        }));

        trials
    }

    /// Every differential specimen either carries an oracle or says why it does not, no
    /// vendor material carries one, and no oracle outlives its specimen.
    ///
    /// This is what keeps a silent omission from reading as a pass: a specimen with no
    /// sidecar produces no `decode` trial, so without this the corpus could lose its
    /// whole oracle and the suite would stay green.
    fn oracles_are_a_decision(root: &Path) {
        // A directory sidecar may answer for every file in it at once.
        let blanket = |dir: &Path| {
            let at = dir.join(DIR_SIDECAR);
            at.exists() && read_oracle(&at).unoracled.is_some()
        };

        let mut missing: Vec<String> = Vec::new();
        for &(dir, ext) in ORACLE_SWEEP {
            let at = root.join(dir);
            if blanket(&at) {
                continue;
            }
            for path in files_with(&at, ext) {
                if !sidecar_of(&path).exists() {
                    missing.push(rel(root, &path));
                }
            }
        }
        assert!(
            missing.is_empty(),
            "specimens with no oracle and no statement that they have none — an absent \
             oracle has to be a decision: {missing:?}",
        );

        // A new panel directory arriving is the case the old per-directory `match` used
        // to catch by panicking on an unknown name.
        let listed: BTreeSet<&str> = ORACLE_SWEEP.iter().map(|(d, _)| *d).collect();
        for entry in std::fs::read_dir(root.join("programs")).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                let dir = format!("programs/{}", name_of(&path));
                assert!(
                    listed.contains(dir.as_str()),
                    "{dir}/ is not in the oracle sweep — add it or move the specimens",
                );
            }
        }

        let mut stray: Vec<String> = Vec::new();
        let mut orphaned: Vec<String> = Vec::new();
        for sidecar in sidecars(root) {
            // The corpus rule, asserted rather than described: a `factory/` path is
            // vendor material and carries no oracle, so one here would be asserting a
            // program name against a knob setting.
            if is_factory(&sidecar) {
                stray.push(rel(root, &sidecar));
            }
            if let Some(specimen) = specimen_of(&sidecar) {
                if !specimen.exists() {
                    orphaned.push(rel(root, &sidecar));
                }
            }
        }
        assert!(
            stray.is_empty(),
            "factory material carries an oracle: {stray:?}",
        );
        assert!(
            orphaned.is_empty(),
            "oracles whose specimen the corpus no longer holds: {orphaned:?}",
        );
    }

    /// A directory's golden dependency table: the ids it lists are the ones its specimens
    /// reference, and no more.
    ///
    /// Each specimen's own sidecar pins its id. What the table adds is the other
    /// direction — that it has not outlived the specimens it was written for — and the
    /// name behind each number.
    fn dependencies(dir: &Path) {
        let table = read_json(&dir.join(DIR_SIDECAR));
        let keyed_by: Vec<String> = table["dependencies"]["keyed_by"]
            .as_array()
            .expect("a dependency table names its key fields")
            .iter()
            .map(|k| k.as_str().unwrap().to_string())
            .collect();
        let field = table["dependencies"]["field"].as_str().unwrap().to_string();

        let mut listed: BTreeMap<Vec<String>, String> = BTreeMap::new();
        for row in table["dependencies"]["table"].as_array().unwrap() {
            let key: Vec<String> = row["key"]
                .as_array()
                .unwrap()
                .iter()
                .map(|k| k.as_str().unwrap().to_string())
                .collect();
            let previous = listed.insert(key.clone(), row["id"].as_str().unwrap().to_string());
            assert!(previous.is_none(), "{key:?} is listed twice");
        }

        let mut covered: BTreeSet<Vec<String>> = BTreeSet::new();
        for path in files_with(dir, "ne5p") {
            if is_skipped(&path) {
                continue;
            }
            let view = oracle_view(&path);
            let key: Vec<String> = keyed_by.iter().map(|k| view[k][0].clone()).collect();
            let id = listed
                .get(&key)
                .unwrap_or_else(|| panic!("{}: nothing in the table for {key:?}", path.display()));
            assert!(
                view[&field].contains(id),
                "{}: {field} is {:?}, the table says {id} for {key:?}",
                path.display(),
                view[&field][1],
            );
            covered.insert(key);
        }

        let stale: Vec<_> = listed
            .keys()
            .filter(|k| !covered.contains(*k))
            .cloned()
            .collect();
        assert!(
            stale.is_empty(),
            "{}: the table lists slots the corpus no longer covers: {stale:?}",
            dir.display(),
        );
    }
}

fn main() {
    #[cfg(feature = "corpus")]
    cases::run();
}
