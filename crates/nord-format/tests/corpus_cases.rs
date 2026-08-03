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
//! NORD_CORPUS_DIR=/path/to/nord-corpus/ne5 \
//!   cargo test -p nord-format --features corpus --test corpus_cases -- programs/organ
//! ```
//!
//! The checks:
//!
//! * `round_trip` — the file re-emits byte for byte, for every format the corpus ships.
//! * `decode` — the filename oracle. **This is the real decode test**: a round trip
//!   passes whatever the fields read, because a panel keeps the bytes it came from.
//! * `mutation` — assigning to a field reaches the bytes, reads back, and moves nothing
//!   outside that panel's span.
//! * `drawbars` — reading nine drawbar nibbles and writing them back is a no-op.
//! * `named_values` — no specimen decodes to a value no component can name.
//! * `as_program` — a live body retagged as a program decodes identically.
//! * `strokes` / `zone_ranges` — the `.nsmp` section chain and its key ranges.
//! * `coverage` — the floors. An empty directory must not read as a pass, and a golden
//!   table must not outlive the specimens it was written for.
//!
//! Tests whose assertion spans the whole corpus rather than one specimen stay in
//! `ne5.rs`; the bit-flip harness is `mutation.rs` and the per-field pins are
//! `decode_snapshot.rs`.

#[cfg(feature = "corpus")]
mod common;

#[cfg(feature = "corpus")]
mod cases {
    use super::common::{
        all_programs, corpus_dir, files_with, is_skipped, read_program, read_sample, read_settings,
        rel,
    };
    use libtest_mimic::{Arguments, Trial};
    use nord_format::electro5::settings::LiveSlot;
    use nord_format::electro5::{
        EqualizerPart, Fx1Type, Fx2Type, Fx3Type, Fx5Type, Instrument, PianoCategory, Routing,
    };
    use nord_format::{electro5, Entity};
    use regex::Regex;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs::read;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};
    use std::str::FromStr;
    use std::sync::OnceLock;

    // -----------------------------------------------------------------------
    // The harness
    // -----------------------------------------------------------------------

    pub fn run() -> ! {
        let args = Arguments::from_args();
        let root = corpus_dir();
        let mut trials = Vec::new();

        let programs = all_programs(&root);
        let sweep: Vec<PathBuf> = programs
            .iter()
            .filter(|p| p.starts_with(root.join("programs")))
            .cloned()
            .collect();

        // Every format the corpus ships re-emits through one code path, so the round
        // trip is one check over all of them rather than one per format.
        for ext in ["ne5p", "ne5s", "ne5t", "ne5l", "nsmp"] {
            for path in files_with(&root, ext) {
                trials.push(trial("round_trip", &root, path, round_trip));
            }
        }

        for path in &sweep {
            trials.push(trial("decode", &root, path.clone(), decode_program));
            trials.push(trial("mutation", &root, path.clone(), mutation));
            trials.push(trial("drawbars", &root, path.clone(), drawbars));
        }
        for path in &programs {
            trials.push(trial("named_values", &root, path.clone(), program_values));
        }

        for path in files_with(&root.join("settings"), "ne5s") {
            if !UNORACLED.contains(&stem(&path).as_str()) {
                trials.push(trial("decode", &root, path.clone(), decode_settings));
            }
        }
        for path in settings_files(&root) {
            trials.push(trial("named_values", &root, path, settings_values));
        }

        for path in files_with(&root, "ne5l") {
            trials.push(trial("as_program", &root, path, live_decodes_as_program));
        }

        for path in files_with(&root.join("samples"), "nsmp") {
            trials.push(trial("strokes", &root, path.clone(), strokes));
            trials.push(trial("zone_ranges", &root, path, zone_ranges));
        }

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

    /// A named trial that is not about one specimen — a floor, or a golden table's
    /// coverage.
    fn aggregate(name: &str, check: impl FnOnce() + Send + 'static) -> Trial {
        Trial::test(format!("coverage/{name}"), move || {
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

    fn regex(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
        cell.get_or_init(|| Regex::new(pattern).expect("test pattern"))
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
    // decode — the filename oracles, one per `programs/` directory
    // -----------------------------------------------------------------------

    fn decode_program(path: &Path) {
        let dir = path
            .parent()
            .and_then(|d| d.file_name())
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_default();

        match dir.as_str() {
            "center_panel" => decode_center_panel(path),
            "equalizer" => decode_equalizer(path),
            "fx" => decode_fx(path),
            "gain" => decode_gain(path),
            "organ" => decode_organ(path),
            "piano" => decode_piano(path),
            "sample" => decode_sample(path),
            other => panic!("{other}/ has no filename oracle — add one or move the specimen"),
        }
    }

    /// A 0..127 knob against the value its filename names.
    ///
    /// ⚠️ At a stop the capture is exact, and mid-travel it is not: the filename records
    /// the value the operator was aiming at, the file the position the knob landed on.
    /// Every knob the corpus captures at mid-travel *with* a centre detent — the three EQ
    /// gains, decay/release at sustain — is exact; the two without one, EQ freq and
    /// sample attack, sit a few counts off. A misplaced bit range moves a value by at
    /// least a factor of two, so the slack costs the oracle nothing.
    fn assert_knob(stored: u8, named: u8, what: &str, path: &Path) {
        const SLACK: u8 = 4;
        let ok = match named {
            0 | 127 => stored == named,
            _ => stored.abs_diff(named) <= SLACK,
        };
        assert!(
            ok,
            "{what} is {stored}, filename says {named}, in file {}",
            path.display()
        );
    }

    fn decode_center_panel(path: &Path) {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = regex(
            &RE,
            r"([ospx])([01])([01])_([0-9.-]+)_([ospx])([01])([01])([01])_([0-9.-]+)_([0-9.-]+)_([0-9.-]+)_([0-9.-]+)_([0-9.-]+)[.]ne5p$",
        );

        let name = name_of(path);
        let m = re
            .captures(&name)
            .unwrap_or_else(|| panic!("invalid file name: {name}"));

        let instrument = |i: usize| match &m[i] {
            "o" => Some(Instrument::Organ),
            "s" => Some(Instrument::Sample),
            "p" => Some(Instrument::Piano),
            "x" => None,
            other => panic!("invalid instrument {other} in {name}"),
        };
        let flag = |i: usize| &m[i] == "1";

        let lower_instrument = instrument(1);
        let lower_sustain = flag(2);
        let lower_control = flag(3);
        let lower_octave_shift = i8::from_str(&m[4]).unwrap();
        let upper_instrument = instrument(5);
        let upper_sustain = flag(6);
        let upper_control = flag(7);
        let transpose_enabled = flag(8);
        let upper_octave_shift = i8::from_str(&m[9]).unwrap();
        let transpose = i8::from_str(&m[10]).unwrap();
        let split = u8::from_str(&m[11]).unwrap();
        let part_mix = (
            f32::from_str(&m[12]).unwrap(),
            f32::from_str(&m[13]).unwrap(),
        );

        let center = read_program(path).schema.center_panel;

        match lower_instrument {
            Some(want) => {
                assert_eq!(center.lower_part, want, "lower instrument in {name}");
                assert!(center.lower_enabled, "lower part enabled in {name}");
            }
            None => assert!(!center.lower_enabled, "lower part enabled in {name}"),
        }
        match upper_instrument {
            Some(want) => {
                assert_eq!(center.upper_part, want, "upper instrument in {name}");
                assert!(center.upper_enabled, "upper part enabled in {name}");
            }
            None => assert!(!center.upper_enabled, "upper part enabled in {name}"),
        }

        assert_eq!(
            center.lower_octave_shift, lower_octave_shift,
            "lower octave shift in {name}"
        );
        assert_eq!(
            center.upper_octave_shift, upper_octave_shift,
            "upper octave shift in {name}"
        );
        assert_eq!(
            center.lower_sustain, lower_sustain,
            "lower sustain in {name}"
        );
        assert_eq!(
            center.upper_sustain, upper_sustain,
            "upper sustain in {name}"
        );
        assert_eq!(
            center.lower_control, lower_control,
            "lower control in {name}"
        );
        assert_eq!(
            center.upper_control, upper_control,
            "upper control in {name}"
        );
        assert_eq!(center.split, split != 0, "split enabled in {name}");
        assert_eq!(
            center.transpose_enabled, transpose_enabled,
            "transpose enabled in {name}"
        );
        assert_eq!(
            center.part_mix.lower().round(),
            part_mix.0.round(),
            "lower part mix in {name}"
        );
        assert_eq!(
            center.part_mix.upper().round(),
            part_mix.1.round(),
            "upper part mix in {name}"
        );
        assert_eq!(center.transpose, transpose, "transpose in {name}");

        if split != 0 {
            assert_eq!(center.split_point as u8, split - 1, "split point in {name}");
        }
    }

    fn decode_gain(path: &Path) {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = regex(&RE, r"([0-9.-]+)[.]ne5p$");

        let name = name_of(path);
        let m = re
            .captures(&name)
            .unwrap_or_else(|| panic!("invalid file name: {name}"));
        let gain = f32::from_str(&m[1]).unwrap();

        assert_eq!(
            read_program(path).schema.center_panel.gain,
            ((gain / 10_f32) * 127_f32).round() as u8,
            "gain in {name}",
        );
    }

    fn decode_fx(path: &Path) {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = regex(
            &RE,
            r"fx([0-9])_([0-9])([0-9])([0-9])_([0-9.-]+)_?([0-9.-]+)?[.]ne5p$",
        );

        let name = name_of(path);
        let m = re
            .captures(&name)
            .unwrap_or_else(|| panic!("invalid file name: {name}"));

        let fx = u8::from_str(&m[1]).unwrap();
        let part_select = u8::from_str(&m[2]).unwrap();
        let switch_enabled = u8::from_str(&m[3]).unwrap() != 0;
        let fx_type = u8::from_str(&m[4]).unwrap();
        let fx_value = f32::from_str(&m[5]).unwrap();
        let fx_value2 = m.get(6).map(|v| f32::from_str(v.as_str()).unwrap());

        let effects = read_program(path).schema.effects_panel;
        let routing = || {
            Routing::from_panel(part_select).unwrap_or_else(|| panic!("bad part select in {name}"))
        };

        match fx {
            1 => {
                assert_eq!(effects.fx1, routing(), "fx1 part select in {name}");
                assert_eq!(effects.fx1_control, switch_enabled, "fx1 control in {name}");
                assert_eq!(
                    effects.fx1_rate,
                    ((fx_value / 10_f32) * 127_f32).floor() as u8,
                    "fx1 rate in {name}"
                );
                assert_eq!(
                    effects.fx1_type,
                    match fx_type {
                        0 => Fx1Type::Pan1,
                        1 => Fx1Type::Pan2,
                        2 => Fx1Type::Pan1And2,
                        3 => Fx1Type::Wah,
                        4 => Fx1Type::Rm,
                        5 => Fx1Type::Trem1,
                        6 => Fx1Type::Trem2,
                        7 => Fx1Type::Trem1And2,
                        a => panic!("unknown fx1 type {a} in {name}"),
                    },
                    "fx1 type in {name}"
                );
            }
            2 => {
                assert_eq!(effects.fx2, routing(), "fx2 part select in {name}");
                assert_eq!(effects.fx2_deep, switch_enabled, "fx2 deep in {name}");
                assert_eq!(
                    effects.fx2_rate,
                    fx_value.floor() as u8,
                    "fx2 rate in {name}"
                );
                assert_eq!(
                    effects.fx2_type,
                    match fx_type {
                        0 => Fx2Type::Flanger,
                        1 => Fx2Type::Chorus1,
                        2 => Fx2Type::Chorus2,
                        3 => Fx2Type::Vibe,
                        4 => Fx2Type::Phaser1,
                        5 => Fx2Type::Phaser2,
                        a => panic!("unknown fx2 type {a} in {name}"),
                    },
                    "fx2 type in {name}"
                );
            }
            3 => {
                assert_eq!(effects.fx3, routing(), "fx3 part select in {name}");
                assert_eq!(
                    effects.fx3_compression.as_u8() as f32,
                    fx_value,
                    "fx3 compression in {name}"
                );
                assert_eq!(
                    effects.fx3_compression.as_u8() > 0,
                    switch_enabled,
                    "fx3 drive on in {name}"
                );
                assert_eq!(
                    effects.fx3_type,
                    match fx_type {
                        0 => Fx3Type::None_,
                        1 => Fx3Type::Twin,
                        2 => Fx3Type::Rotary,
                        3 => Fx3Type::Comp,
                        4 => Fx3Type::Small,
                        5 => Fx3Type::Jc,
                        a => panic!("unknown fx3 type {a} in {name}"),
                    },
                    "fx3 type in {name}"
                );
            }
            4 => {
                assert_eq!(effects.fx4, routing(), "fx4 part select in {name}");
                assert_eq!(
                    effects.fx4_ping_pong, switch_enabled,
                    "fx4 ping pong in {name}"
                );
                assert_eq!(
                    effects.fx4_moisture.as_u8() as f32,
                    ((fx_value / 10_f32) * 127_f32).floor(),
                    "fx4 moisture in {name}"
                );
                assert_eq!(
                    effects.fx4_tempo.as_u8() as f32,
                    fx_value2.unwrap().floor(),
                    "fx4 tempo in {name}"
                );
                assert_eq!(effects.fx4_feedback, fx_type, "fx4 type in {name}");
            }
            5 => {
                assert_eq!(effects.fx5, part_select == 1, "fx5 part select in {name}");
                assert_eq!(
                    effects.fx5_moisture.as_u8() as f32,
                    fx_value,
                    "fx5 moisture in {name}"
                );
                assert_eq!(
                    effects.fx5_type,
                    match fx_type {
                        0 => Fx5Type::Stage,
                        1 => Fx5Type::HallSoft,
                        2 => Fx5Type::Hall,
                        3 => Fx5Type::Room,
                        4 => Fx5Type::StageSoft,
                        a => panic!("unknown fx5 type {a} in {name}"),
                    },
                    "fx5 type in {name}"
                );
            }
            other => panic!("unknown fx {other} in {name}"),
        }
    }

    /// `a_bbbcccdddeee.ne5p` — a: part select, then bass, freq, freq gain and treble. See
    /// `programs/equalizer/README.md`.
    ///
    /// The panel's "off" position is a bit of its own, so the filename's four positions
    /// are two fields: `equalizer_on`, and which part the EQ reaches when it is on.
    fn decode_equalizer(path: &Path) {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = regex(
            &RE,
            r"([0-9]+)_([0-9]{3})([0-9]{3})([0-9]{3})([0-9]{3})[.]ne5p$",
        );

        let name = name_of(path);
        let m = re
            .captures(&name)
            .unwrap_or_else(|| panic!("invalid file name: {name}"));

        let part_select = u8::from_str(&m[1]).unwrap();
        let bass = u8::from_str(&m[2]).unwrap();
        let freq = u8::from_str(&m[3]).unwrap();
        let freq_gain = u8::from_str(&m[4]).unwrap();
        let treble = u8::from_str(&m[5]).unwrap();

        let eq = read_program(path).schema.effects_panel;

        assert_eq!(eq.equalizer_on, part_select != 0, "equalizer on in {name}");
        // Off leaves the part where it was, so it is the one position the filename does
        // not name.
        if part_select != 0 {
            assert_eq!(
                eq.equalizer_part,
                match part_select {
                    1 => EqualizerPart::Lower,
                    2 => EqualizerPart::Upper,
                    3 => EqualizerPart::Both,
                    a => panic!("unknown equalizer part {a} in {name}"),
                },
                "equalizer part in {name}"
            );
        }

        assert_knob(eq.equalizer_bass.as_u8(), bass, "equalizer bass", path);
        assert_knob(eq.equalizer_freq.as_u8(), freq, "equalizer freq", path);
        assert_knob(
            eq.equalizer_freq_gain.as_u8(),
            freq_gain,
            "equalizer freq gain",
            path,
        );
        assert_knob(
            eq.equalizer_treble.as_u8(),
            treble,
            "equalizer treble",
            path,
        );
    }

    /// `abc_dd_eee_fggg.ne5p` — a: part, b: dynamics, c: filter, dd: sample number, eee:
    /// attack, f/ggg: decay-release. See `programs/sample/README.md`.
    ///
    /// `dd` is read from both ends at once: it is the panel number, and it selects the
    /// golden dependency id the instrument puts on the wire for that slot. `number`,
    /// `id`, `dynamics` and `filter` are packed shoulder to shoulder in one word, so a
    /// shift off by a bit smears one into the next.
    fn decode_sample(path: &Path) {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = regex(
            &RE,
            r"([0-9])([0-9])([0-9])_([a-fA-F0-9]{2})_([0-9]{3})_([dsr])([0-9]{3})[.]ne5p$",
        );

        let name = name_of(path);
        let m = re
            .captures(&name)
            .unwrap_or_else(|| panic!("invalid file name: {name}"));

        let part_select = u8::from_str(&m[1]).unwrap();
        let dynamics = u8::from_str(&m[2]).unwrap();
        let filter = &m[3] == "1";
        // The panel shows a 1-based number; the field stores the slot.
        let number = panel_number(&m[4]) - 1;
        let attack = u8::from_str(&m[5]).unwrap();
        let decay_release_type = m[6].to_string();
        let decay_release = u8::from_str(&m[7]).unwrap();

        let schema = read_program(path).schema;
        let center = &schema.center_panel;
        let (part, enabled) = match part_select {
            0 => (center.lower_part, center.lower_enabled),
            1 => (center.upper_part, center.upper_enabled),
            a => panic!("unknown part {a} in {name}"),
        };
        assert_eq!(
            (part, enabled),
            (Instrument::Sample, true),
            "sample part in {name}"
        );

        let sample = &schema.sample_panel;
        assert_eq!(sample.dynamics, dynamics, "sample dynamics in {name}");
        assert_eq!(sample.filter, filter, "sample filter in {name}");
        assert_eq!(sample.number, number, "sample number in {name}");
        assert_knob(sample.attack.as_u8(), attack, "sample attack", path);
        // One knob, three regimes: decay below the midpoint, sustain at it, release
        // above. The filename names both the number and which side of the midpoint it is
        // meant to fall on.
        assert_knob(
            sample.decay_release.as_u8(),
            decay_release,
            "sample decay/release",
            path,
        );
        let regime = match decay_release {
            0..=63 => "d",
            64 => "s",
            _ => "r",
        };
        assert_eq!(
            regime, decay_release_type,
            "decay/release {decay_release} is not a {decay_release_type} in {name}"
        );

        let id = sample_golden()
            .get(&number)
            .unwrap_or_else(|| panic!("no golden id for sample number {number}, from {name}"));
        assert_eq!(sample.id, *id, "sample id in {name}");
    }

    /// `abcd_ee_ff.ne5p` — a: part, b: acoustics, c: mono, d: touch, ee: type, ff: model.
    /// See `programs/piano/README.md`.
    fn decode_piano(path: &Path) {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = regex(
            &RE,
            r"([0-9])([0-3])([01])([0-3])_([0-9]{2})_([0-9A-Fa-f]{2})\.ne5p$",
        );

        let name = name_of(path);
        let m = re
            .captures(&name)
            .unwrap_or_else(|| panic!("invalid file name: {name}"));

        let acoustics = u8::from_str(&m[2]).unwrap();
        let mono = &m[3] == "1";
        let touch = u8::from_str(&m[4]).unwrap();
        let category = piano_category(u8::from_str(&m[5]).unwrap());

        let piano = read_program(path).schema.piano_panel;

        assert_eq!(piano.category, category, "category in {name}");
        assert_eq!(piano.acoustics, acoustics, "acoustics in {name}");
        assert_eq!(piano.mono, mono, "mono in {name}");
        assert_eq!(piano.touch, touch, "touch in {name}");

        // Clav's model field is a variant code (`0A`, `0d`) rather than a slot number —
        // those two specimens differ only in `clav_model` and both sit in model slot 0.
        if category != PianoCategory::Clavinet {
            // The panel shows a 1-based model number; the field stores the slot.
            assert_eq!(
                piano.piano_model,
                panel_number(&m[6]) - 1,
                "piano_model in {name}"
            );
        }

        let slot = (piano.category, piano.piano_model.as_u8());
        let (id, piano_name) = piano_golden()
            .get(&slot)
            .unwrap_or_else(|| panic!("no golden id for slot {slot:?}, from {name}"));

        // The value, not just its stability: this is the number the instrument puts on
        // the wire for "{piano_name}".
        assert_eq!(
            piano.id, *id,
            "piano id in {name} should reference {piano_name}",
        );
    }

    /// The three filename shapes the organ corpus uses:
    ///
    /// ```text
    /// type-A  P d c t s v y DDDDDDDDD   (16 digits: 7 B3 toggle fields + 9 drawbars)
    /// type-B  PMrs_DDDDDDDDD            (preset, model, rot_speed, rot_stop + 9 drawbars)
    /// type-C  PMrs_ctsvy                (preset, model, rot_speed, rot_stop + 5 perc/vib)
    /// ```
    fn decode_organ(path: &Path) {
        use nord_format::electro5::{OrganModel, PercSpeed, VibChorus};

        // Filename drawbar char -> physical position (0..=8). Digits and the two letter
        // ranges all encode the same nine physical positions; only the display "real"
        // value differs (a..i => real 0, j..r => real 1).
        fn physical(c: u8) -> u8 {
            match c {
                b'0'..=b'8' => c - b'0',
                b'a'..=b'i' => c - b'a',
                b'j'..=b'r' => c - b'j',
                _ => panic!("bad drawbar char: {}", c as char),
            }
        }

        // Filename model digit -> (model, on-disk value == physical position?). B3/Vox/
        // Pipe store the physical bar position, so their drawbars are asserted directly.
        // B3-bass (1) remaps its bass bars and Farfisa (4) quantizes intermediate values
        // on disk (e.g. physical 5 -> 4), so their exact values aren't asserted yet — the
        // bytes still round-trip.
        fn model_of(d: u8) -> (OrganModel, bool) {
            match d {
                0 => (OrganModel::B3, true),
                1 => (OrganModel::B3, false),
                2 => (OrganModel::Pipe, true),
                3 => (OrganModel::Vox, true),
                4 => (OrganModel::Farfisa, false),
                _ => panic!("unknown organ model digit {d}"),
            }
        }

        // Filename vib-type digit (0..5) -> mode. Filename order is v2,c2,v3,c3,v1,c1.
        fn vib_of(i: u8) -> VibChorus {
            use VibChorus::*;
            [V2, C2, V3, C3, V1, C1][i as usize]
        }
        fn speed_of(e: u8) -> PercSpeed {
            use PercSpeed::*;
            [Off, Soft, Fast, Both][e as usize]
        }

        static TYPE_A: OnceLock<Regex> = OnceLock::new();
        static TYPE_B: OnceLock<Regex> = OnceLock::new();
        static TYPE_C: OnceLock<Regex> = OnceLock::new();
        let type_a = regex(&TYPE_A, r"^(\d)(\d)(\d)(\d)(\d)(\d)(\d)([0-8]{9})\.ne5p$");
        let type_b = regex(&TYPE_B, r"^(\d)(\d)(\d)(\d)_([0-8a-r]{9})\.ne5p$");
        let type_c = regex(&TYPE_C, r"^(\d)(\d)(\d)(\d)_(\d)(\d)(\d)(\d)(\d)\.ne5p$");

        let name = name_of(path);
        let dig = |m: &regex::Captures, i: usize| m[i].parse::<u8>().unwrap();

        enum Toggles {
            /// type-A: full B3 percussion + vibrato state.
            B3 {
                perc_on: bool,
                perc_third: bool,
                perc_speed: PercSpeed,
                vib_on: bool,
                vib_type: VibChorus,
            },
            /// type-C: Vox/Farfisa vibrato only — they have no percussion.
            Vib { vib_on: bool, vib_type: VibChorus },
            /// type-B drawbar specimens carry no toggle info.
            None,
        }

        let (model, preset, drawbars, physical_storage, toggles) =
            if let Some(m) = type_a.captures(&name) {
                let toggles = Toggles::B3 {
                    perc_on: dig(&m, 3) == 1,
                    perc_third: dig(&m, 4) == 1,
                    perc_speed: speed_of(dig(&m, 5)),
                    vib_on: dig(&m, 6) == 1,
                    vib_type: vib_of(dig(&m, 7)),
                };
                (
                    OrganModel::B3,
                    dig(&m, 1),
                    Some(m[8].to_string()),
                    true,
                    toggles,
                )
            } else if let Some(m) = type_b.captures(&name) {
                let (model, phys) = model_of(dig(&m, 2));
                (
                    model,
                    dig(&m, 1),
                    Some(m[5].to_string()),
                    phys,
                    Toggles::None,
                )
            } else if let Some(m) = type_c.captures(&name) {
                let (model, _) = model_of(dig(&m, 2));
                let toggles = Toggles::Vib {
                    vib_on: dig(&m, 8) == 1,
                    vib_type: vib_of(dig(&m, 9)),
                };
                (model, dig(&m, 1), None, false, toggles)
            } else {
                panic!("unrecognized organ file name: {name}");
            };

        let program = read_program(path);
        let organ = &program.schema.organ_panel;

        assert_eq!(organ.preset(model), preset, "preset in {name}");

        if let (Some(chars), true) = (drawbars.as_ref(), physical_storage) {
            let expected: Vec<u8> = chars.bytes().map(physical).collect();
            assert_eq!(
                organ.drawbars(model, preset).as_slice(),
                expected.as_slice(),
                "drawbar decode in {name} ({model:?} preset {preset})",
            );
        }

        match toggles {
            Toggles::B3 {
                perc_on,
                perc_third,
                perc_speed,
                vib_on,
                vib_type,
            } => {
                assert_eq!(organ.b3_perc_on(preset), perc_on, "b3 perc_on in {name}");
                assert_eq!(organ.b3_perc_third(), perc_third, "b3 perc_third in {name}");
                assert_eq!(organ.b3_perc_speed(), perc_speed, "b3 perc_speed in {name}");
                assert_eq!(organ.vib_on(model, preset), vib_on, "b3 vib_on in {name}");
                assert_eq!(
                    organ.vib_type(model),
                    Some(vib_type),
                    "b3 vib_type in {name}"
                );
            }
            Toggles::Vib { vib_on, vib_type } => {
                assert_eq!(organ.vib_on(model, preset), vib_on, "vib_on in {name}");
                assert_eq!(organ.vib_type(model), Some(vib_type), "vib_type in {name}");
            }
            Toggles::None => {}
        }

        // b3+bass preset 1 keeps its two bass drawbars outside the nine-nibble block, so
        // `drawbars()` cannot see them. This is the real-data counterpart to the unit
        // tests in `program.rs`: those pin the bit layout, this pins it to specimens
        // captured off the instrument.
        if is_b3_bass_preset1(&name) {
            let bars = name.split_once('_').unwrap().1.as_bytes();
            let want = [bars[0] - b'0', bars[1] - b'0'];
            let got = organ.b3_bass_drawbars();
            assert_eq!(got, want, "bass drawbars in {name}");

            // The main block's first two nibbles are stale in this mode and must not be
            // mistaken for the bass values — assert we are genuinely reading elsewhere.
            let main = organ.drawbars(OrganModel::B3, 1);
            assert!(
                want == [0, 0] || (main[0], main[1]) != (want[0], want[1]),
                "{name}: bass values also appear in the main block — offsets may be wrong",
            );
        }
    }

    /// Type-B, model digit 1, preset 1: the only shape that carries a bass manual.
    fn is_b3_bass_preset1(name: &str) -> bool {
        let Some((head, rest)) = name.split_once('_') else {
            return false;
        };
        let Some(bars) = rest.strip_suffix(".ne5p") else {
            return false;
        };
        head.len() == 4
            && bars.len() == 9
            && bars.chars().all(|c| c.is_ascii_digit())
            && head.starts_with("11")
    }

    /// A two-character panel number as the Electro 5 displays it: the tens digit runs
    /// `0`..`9` then `A`..`F`, the units digit `0`..`9`, so `F9` is 159.
    fn panel_number(text: &str) -> u8 {
        let bytes = text.as_bytes();
        assert_eq!(bytes.len(), 2, "not a panel number: {text}");

        let tens = match bytes[0] {
            c @ b'0'..=b'9' => c - b'0',
            c @ b'A'..=b'F' => c - b'A' + 10,
            c @ b'a'..=b'f' => c - b'a' + 10,
            c => panic!("bad panel number digit: {}", c as char),
        };
        let units = bytes[1] - b'0';
        assert!(units < 10, "bad panel number digit: {text}");

        tens * 10 + units
    }

    // -----------------------------------------------------------------------
    // Golden dependency ids
    // -----------------------------------------------------------------------
    //
    // Both panels carry a 32-bit reference to the piano (`.npno`) or sample (`.nsmp`)
    // the program needs, in bits 41..=10 of the panel word. Everything else in those
    // panels is a *slot coordinate* — the piano category + model dials, the Samp Lib
    // number — which moves when the instrument's library changes. The id is the stable
    // key: it is what resolves the song -> program -> piano chain, and what Nord Sound
    // Manager checks before offering a Restore.
    //
    // The golden ids are not self-referential: each was read off the USB captures, where
    // the vendor protocol transmits the same id as a plain big-endian u32 immediately
    // followed by a length-prefixed piano name (see `usb/program/relink_piano_*`). Byte
    // alignment there is what fixes the shift — a decode off by even one bit produces
    // values that appear nowhere on the wire.

    /// `(category, model, id, name)` for every piano slot the `programs/piano` specimens
    /// select. The names are the shipped `.npno` basenames.
    const PIANO_IDS: [(PianoCategory, u8, u32, &str); 11] = [
        (
            PianoCategory::Grand,
            0,
            0xd303_b5f2,
            "Royal Grand 3D YaS6 XL 5.4",
        ),
        (
            PianoCategory::Grand,
            5,
            0x4ca6_ab08,
            "Electric Grand 1 CP80  5.3",
        ),
        (
            PianoCategory::Upright,
            0,
            0x645f_053e,
            "Grand Upright YaU3 Lrg 5.4",
        ),
        (
            PianoCategory::Upright,
            5,
            0x42a7_a3a3,
            "HonkyTonkUpright      Sml 5.3",
        ),
        (
            PianoCategory::EPiano1,
            0,
            0x6434_2577,
            "EPiano 1    Mk I Low Deep  5.3",
        ),
        (
            PianoCategory::EPiano1,
            9,
            0x19c5_f749,
            "EP8 Nefertiti Lrg 6.0",
        ),
        (
            PianoCategory::EPiano2,
            0,
            0xd1ac_cddd,
            "DX7 FullTines  Lrg 5.4",
        ),
        (
            PianoCategory::EPiano2,
            2,
            0x17a8_d178,
            "Ballad EP1  Sml 5.2",
        ),
        (PianoCategory::Clavinet, 0, 0x9bed_fa45, "Clavinet D6  5.0"),
        (
            PianoCategory::Harpsichord,
            0,
            0xf121_ce36,
            "Ital Harpsich 1B Long Stri 5.0",
        ),
        (
            PianoCategory::Harpsichord,
            2,
            0x1251_b69a,
            "Ital Harpsich 1D Lute 5.0",
        ),
    ];

    /// `(samp lib number, id)` for the sample slots `programs/sample` selects. As above,
    /// the ids are the ones the vendor protocol puts on the wire.
    const SAMPLE_IDS: [(u8, u32); 3] = [(0, 0x47c8_b4f8), (41, 0x89be_e289), (158, 0x0ac2_1363)];

    fn piano_golden() -> &'static BTreeMap<(PianoCategory, u8), (u32, &'static str)> {
        static GOLDEN: OnceLock<BTreeMap<(PianoCategory, u8), (u32, &'static str)>> =
            OnceLock::new();
        GOLDEN.get_or_init(|| {
            PIANO_IDS
                .iter()
                .map(|&(category, model, id, name)| ((category, model), (id, name)))
                .collect()
        })
    }

    fn sample_golden() -> &'static BTreeMap<u8, u32> {
        static GOLDEN: OnceLock<BTreeMap<u8, u32>> = OnceLock::new();
        GOLDEN.get_or_init(|| SAMPLE_IDS.iter().copied().collect())
    }

    /// Filename type digit -> the category it names.
    fn piano_category(ee: u8) -> PianoCategory {
        match ee {
            1 => PianoCategory::EPiano1,
            2 => PianoCategory::EPiano2,
            3 => PianoCategory::Clavinet,
            4 => PianoCategory::Harpsichord,
            5 => PianoCategory::Grand,
            6 => PianoCategory::Upright,
            _ => panic!("bad piano type digit: {ee}"),
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

        // The tag is the whole difference: the CRC covers `0x2c..` and never sees the
        // header, so retagging in place leaves a valid file.
        let mut bytes = read(path).unwrap();
        bytes[0x08..0x0c].copy_from_slice(electro5::program::FORMAT.as_bytes());

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
    // Settings
    // -----------------------------------------------------------------------

    /// `(specimen stem, field, the value it must decode to)`. **The filename is the
    /// oracle**: a round trip proves the bytes survive, only this proves the decode reads
    /// the setting the operator actually changed.
    ///
    /// Values are the field's own `Debug` rendering, which is what `nord inspect` prints.
    ///
    /// Stems are keyed through [`oracle_key`], so a negative value spelled `--6` and one
    /// spelled `-6` are the same row.
    const SETTINGS_ORACLE: &[(&str, &str, &str)] = &[
        ("b3-key-bounce-mode-off", "b3_key_bounce", "false"),
        ("b3-key-bounce-mode-on", "b3_key_bounce", "true"),
        ("b3-key-click-lvl-low", "b3_key_click_level", "Low"),
        ("b3-key-click-lvl-normal", "b3_key_click_level", "Normal"),
        ("b3-key-click-lvl-high", "b3_key_click_level", "High"),
        ("b3-key-click-lvl-higher", "b3_key_click_level", "Higher"),
        ("b3-perc-DB9-mute-mode-off", "b3_perc_db9_mute", "false"),
        ("b3-perc-DB9-mute-mode-on", "b3_perc_db9_mute", "true"),
        ("b3-perc-decay-fast-short", "b3_perc_decay_fast", "Short"),
        ("b3-perc-decay-fast-medium", "b3_perc_decay_fast", "Medium"),
        ("b3-perc-decay-fast-long", "b3_perc_decay_fast", "Long"),
        ("b3-perc-decay-slow-short", "b3_perc_decay_slow", "Short"),
        ("b3-perc-decay-slow-medium", "b3_perc_decay_slow", "Medium"),
        ("b3-perc-decay-slow-long", "b3_perc_decay_slow", "Long"),
        ("b3-perc-vol-normal-low", "b3_perc_volume_normal", "Low"),
        (
            "b3-perc-vol-normal-medium",
            "b3_perc_volume_normal",
            "Medium",
        ),
        ("b3-perc-vol-normal-high", "b3_perc_volume_normal", "High"),
        // `slow` in the filename is the panel's *low*; the soft volume has no speed.
        ("b3-perc-vol-soft-slow", "b3_perc_volume_soft", "Low"),
        ("b3-perc-vol-soft-medium", "b3_perc_volume_soft", "Medium"),
        ("b3-perc-vol-soft-high", "b3_perc_volume_soft", "High"),
        ("b3-tonewheel-mode-clean", "b3_tonewheel_mode", "Clean"),
        (
            "b3-tonewheel-mode-vintage1",
            "b3_tonewheel_mode",
            "Vintage1",
        ),
        (
            "b3-tonewheel-mode-vintage2",
            "b3_tonewheel_mode",
            "Vintage2",
        ),
        (
            "b3-tonewheel-mode-vintage3",
            "b3_tonewheel_mode",
            "Vintage3",
        ),
        ("ctr-pedal-gain-1", "ctrl_pedal_gain", "1"),
        ("ctr-pedal-gain-10", "ctrl_pedal_gain", "10"),
        ("ctr-pedal-type-Roland-EV7", "ctrl_pedal_type", "RolandEv7"),
        ("ctrl-pedal-type-yamaha-FC7", "ctrl_pedal_type", "YamahaFc7"),
        ("ctr-pedal-type-Korg-EXP2", "ctrl_pedal_type", "KorgExp2"),
        ("ctr-pedal-type-Korg-XVP10", "ctrl_pedal_type", "KorgXvp10"),
        (
            "ctr-pedal-type-Boss-FV500L",
            "ctrl_pedal_type",
            "BossFv500L",
        ),
        ("ctr-pedal-type-Fatar-SL", "ctrl_pedal_type", "FatarSl"),
        // `50c` with no sign is minus fifty; the plus is spelled out when it is one.
        ("fine-tune-50c", "fine_tune", "-50"),
        ("fine-tune-0c", "fine_tune", "0"),
        ("fine-tune-+50c", "fine_tune", "50"),
        ("global-transpose--6", "global_transpose", "-6"),
        ("global-transpose-1", "global_transpose", "-1"),
        ("global-transpose-+1", "global_transpose", "1"),
        ("global-transpose-+6", "global_transpose", "6"),
        ("midi-chan-global-off", "global_channel", "off"),
        ("midi-chan-global-1", "global_channel", "1"),
        ("midi-chan-global-2", "global_channel", "2"),
        ("midi-chan-global-16", "global_channel", "16"),
        ("midi-chan-lower-recv-off", "lower_receive_channel", "off"),
        ("midi-chan-lower-recv-1", "lower_receive_channel", "1"),
        ("midi-chan-lower-recv-12", "lower_receive_channel", "12"),
        ("midi-chan-lower-recv-16", "lower_receive_channel", "16"),
        ("midi-chan-upper-recv-off", "upper_receive_channel", "off"),
        ("midi-chan-upper-recv-1", "upper_receive_channel", "1"),
        ("midi-chan-upper-recv-2", "upper_receive_channel", "2"),
        ("midi-chan-upper-recv-16", "upper_receive_channel", "16"),
        ("midi-chan-upper-split-off", "upper_split_channel", "off"),
        ("midi-chan-upper-split-1", "upper_split_channel", "1"),
        ("midi-chan-upper-split-2", "upper_split_channel", "2"),
        ("midi-chan-upper-split-16", "upper_split_channel", "16"),
        ("midi-ctrl-change-mode-off", "control_change_mode", "Off"),
        ("midi-ctrl-change-mode-send", "control_change_mode", "Send"),
        (
            "midi-ctrl-change-mode-recv",
            "control_change_mode",
            "Receive",
        ),
        (
            "midi-ctrl-change-mode-send-recv",
            "control_change_mode",
            "SendReceive",
        ),
        ("midi-prgrm-change-mode-off", "program_change_mode", "Off"),
        ("midi-prgrm-change-mode-send", "program_change_mode", "Send"),
        (
            "midi-prgrm-change-mode-recv",
            "program_change_mode",
            "Receive",
        ),
        (
            "midi-prgrm-change-mode-send-recv",
            "program_change_mode",
            "SendReceive",
        ),
        ("midi-transpose-at-midi-in", "transpose_at", "MidiIn"),
        ("midi-transpose-at-midi-out", "transpose_at", "MidiOut"),
        ("organ-b3-trig-mode-normal", "b3_trig_mode", "Normal"),
        ("organ-b3-trig-mode-fast", "b3_trig_mode", "Fast"),
        ("output-routing-mode-stereo", "output_routing", "Stereo"),
        (
            "output-routing-mode-pLLpUR",
            "output_routing",
            "LowerLeftUpperRight",
        ),
        ("piano-str-res-lvl--6db", "piano_string_resonance", "-6"),
        ("piano-str-res-lvl-0db", "piano_string_resonance", "0"),
        ("piano-str-res-lvl-+6db", "piano_string_resonance", "6"),
        (
            "rotary-balance-bass-horn-70-30",
            "rotary_balance",
            "Bass70Horn30",
        ),
        (
            "rotary-balance-bass-horn-60-40",
            "rotary_balance",
            "Bass60Horn40",
        ),
        (
            "rotary-balance-bass-horn-50-50",
            "rotary_balance",
            "Bass50Horn50",
        ),
        (
            "rotary-balance-bass-horn-40-60",
            "rotary_balance",
            "Bass40Horn60",
        ),
        (
            "rotary-balance-bass-horn-30-70",
            "rotary_balance",
            "Bass30Horn70",
        ),
        // The panel calls value 0 *closed*; the filename spells it half-closed.
        ("rotary-ctrl-type-half-closed", "rotary_ctrl_type", "Closed"),
        ("rotary-ctrl-type-open", "rotary_ctrl_type", "Open"),
        ("rotary-ctrl-type-half-moon", "rotary_ctrl_type", "HalfMoon"),
        ("rotary-horn-acc-low", "rotary_horn_acceleration", "Low"),
        (
            "rotary-horn-acc-normal",
            "rotary_horn_acceleration",
            "Normal",
        ),
        ("rotary-horn-acc-high", "rotary_horn_acceleration", "High"),
        ("rotary-horn-speed-low", "rotary_horn_speed", "Low"),
        ("rotary-horn-speed-normal", "rotary_horn_speed", "Normal"),
        ("rotary-horn-speed-high", "rotary_horn_speed", "High"),
        ("rotary-pedal-mode-hold", "rotary_pedal_mode", "Hold"),
        ("rotary-pedal-mode-toggle", "rotary_pedal_mode", "Toggle"),
        ("rotary-rotor-acc-low", "rotary_rotor_acceleration", "Low"),
        (
            "rotary-rotor-acc-normal",
            "rotary_rotor_acceleration",
            "Normal",
        ),
        ("rotary-rotor-acc-high", "rotary_rotor_acceleration", "High"),
        ("rotary-rotor-speed-normal", "rotary_rotor_speed", "Normal"),
        ("rotary-rotor-speed-high", "rotary_rotor_speed", "High"),
        (
            "rotary-speaker-type-122",
            "rotary_speaker_type",
            "Rotary122",
        ),
        (
            "rotary-speaker-type-122close",
            "rotary_speaker_type",
            "Rotary122Close",
        ),
        ("sus-pedal-auto=open", "sustain_pedal_type", "Auto"),
        ("sus-pedal-closed", "sustain_pedal_type", "Closed"),
        ("sus-pedal-open", "sustain_pedal_type", "Open"),
        ("sus-pedal-mode-sus", "sustain_pedal_mode", "Sustain"),
        (
            "sus-pedal-mode-sus+rotor-hold",
            "sustain_pedal_mode",
            "SustainRotorHold",
        ),
        (
            "sus-pedal-mode-sus+rotor-toggle",
            "sustain_pedal_mode",
            "SustainRotorToggle",
        ),
    ];

    /// `(specimen stem, the sibling it is byte-identical to)`.
    ///
    /// Every one of these was captured to change a setting and changed nothing. That
    /// identity is the finding, so it is asserted rather than skipped: a corrected capture
    /// makes this fail, which is when the decode has something new to learn.
    ///
    /// * `mem-protect-*` and `midi-local-ctrl-mode-*` are the two catalogued settings with
    ///   no decoded home — toggling either moves no bit of the body.
    /// * `rotary-rotor-speed-low` is a duplicate of the `high` capture, so `low` is the one
    ///   rate value the sweep never reaches.
    const SETTINGS_UNMOVED: &[(&str, &str)] = &[
        ("mem-protect-on", "baseline"),
        ("mem-protect-off", "baseline"),
        ("midi-local-ctrl-mode-on", "midi-local-ctrl-mode-off"),
        ("midi-local-ctrl-mode-off", "midi-local-ctrl-mode-on"),
        ("rotary-rotor-speed-low", "rotary-rotor-speed-high"),
    ];

    /// The reboot specimens name no menu setting: they were captured after a power cycle
    /// with nothing touched, so what the filename records is where the instrument was.
    /// `5_2` is program bank 5 slot 2, `live-2` is Live mode on LIVE 2. Locations are
    /// compared zero-based, as stored.
    ///
    /// The Live slot and the program are each *retained* across a change of the other,
    /// which is why the `live-*` specimens all name program `5:2` and `program-5_3` still
    /// sits on LIVE 3 — expectations here follow capture order, not the filename alone.
    ///
    /// `None` for the program is a specimen whose panel state was not recorded; only what
    /// the capture note does pin is asserted.
    const SELECTION_ORACLE: &[(&str, bool, LiveSlot, Option<(u16, u16)>)] = &[
        ("reboot-program-5_1", false, LiveSlot::Live1, Some((4, 0))),
        ("reboot-program-5_2", false, LiveSlot::Live1, Some((4, 1))),
        ("reboot-program-5_3", false, LiveSlot::Live3, Some((4, 2))),
        ("reboot-live-1", true, LiveSlot::Live1, Some((4, 1))),
        ("reboot-live-2", true, LiveSlot::Live2, Some((4, 1))),
        ("reboot-live-3", true, LiveSlot::Live3, Some((4, 1))),
        (
            "reboot-sus-connected-off",
            true,
            LiveSlot::Live3,
            Some((4, 1)),
        ),
        (
            "reboot-sus-connected-on",
            true,
            LiveSlot::Live3,
            Some((4, 1)),
        ),
        (
            "reboot-sus-disconnected",
            true,
            LiveSlot::Live3,
            Some((4, 1)),
        ),
        ("reboot-ctrl-connected", true, LiveSlot::Live3, Some((4, 1))),
        (
            "reboot-ctrl-disconnected",
            true,
            LiveSlot::Live3,
            Some((4, 1)),
        ),
        // Taken a minute after `reboot-ctrl-disconnected` in whatever state the panel was
        // left in, so the program it points at is not known. It is kept for the one thing
        // it does show: the Live slot survives leaving Live mode.
        ("reboot-1", false, LiveSlot::Live3, None),
    ];

    /// The pedal specimens were captured in the state `reboot-live-3` left behind, so a
    /// pedal moving any bit of the body would show up as a difference from it.
    const PEDAL_UNMOVED: &[&str] = &[
        "reboot-sus-connected-off",
        "reboot-sus-connected-on",
        "reboot-sus-disconnected",
        "reboot-ctrl-connected",
        "reboot-ctrl-disconnected",
    ];

    /// Specimens with no filename oracle, listed so the absence is a decision rather than
    /// an omission. `baseline` is every field at once; `decode_snapshot.rs` pins it.
    const UNORACLED: &[&str] = &["baseline"];

    /// A specimen stem as [`SETTINGS_ORACLE`] keys it.
    ///
    /// The corpus spells a negative value both ways — `global-transpose-6` and
    /// `global-transpose--6` are the same capture, the doubled dash disambiguating it from
    /// the `+6` sibling. Collapsing the pair keeps one row per specimen.
    fn oracle_key(stem: &str) -> String {
        stem.replace("--", "-")
    }

    /// Every `.ne5s` the corpus ships: the sweep, the standalone file and the backup's
    /// copy.
    fn settings_files(root: &Path) -> Vec<PathBuf> {
        files_with(root, "ne5s")
    }

    fn decode_settings(path: &Path) {
        use nord_format::panel::Panel;

        let name = stem(path);
        let key = oracle_key(&name);
        let dir = path.parent().unwrap();

        if let Some((_, sibling)) = SETTINGS_UNMOVED.iter().find(|(s, _)| *s == key) {
            assert_eq!(
                read(path).unwrap(),
                read(dir.join(format!("{sibling}.ne5s"))).unwrap(),
                "{name} is no longer identical to {sibling} — the corpus gained a capture \
                 that moves something, so this specimen can now be asserted",
            );
            return;
        }

        if let Some(&(_, live_mode, live_slot, program)) =
            SELECTION_ORACLE.iter().find(|(s, ..)| *s == name)
        {
            let selection = read_settings(path).schema.selection;
            assert_eq!(selection.live_mode, live_mode, "live_mode in {name}");
            assert_eq!(selection.live_slot, live_slot, "live_slot in {name}");
            assert!(!selection.set_list_mode, "set_list_mode in {name}");
            if let Some(program) = program {
                assert_eq!(selection.program.inner(), program, "program in {name}");
            }

            if PEDAL_UNMOVED.contains(&name.as_str()) {
                assert_eq!(
                    read(path).unwrap(),
                    read(dir.join("reboot-live-3.ne5s")).unwrap(),
                    "{name} is no longer identical to reboot-live-3 — a pedal moved \
                     something, so the settings body does carry pedal state after all",
                );
            }
            return;
        }

        let (_, field, want) = SETTINGS_ORACLE
            .iter()
            .find(|(s, ..)| oracle_key(s) == key)
            .unwrap_or_else(|| panic!("{name} has no expected value — add it to the oracle"));

        let values = read_settings(path).schema.panel.field_values();
        let got = values
            .iter()
            .find(|v| v.name == *field)
            .unwrap_or_else(|| panic!("{name}: the panel declares no field {field}"));
        assert_eq!(&got.value, want, "{field} in {name}");
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
    /// they were overridden by hand**, which is what `D7-upperkey` exists to demonstrate.
    ///
    /// So the derivation is the editor's default, not a rule the format enforces: the top
    /// note really is stored, and a reader must take it as read rather than recompute it.
    fn zone_ranges(path: &Path) {
        use nord_format::common::sample::zone;

        /// Specimens whose upper key was deliberately moved off the default.
        const OVERRIDDEN: &[&str] = &["D7-upperkey"];

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

        if OVERRIDDEN.contains(&name.as_str()) {
            assert_ne!(
                stored, derived,
                "{name} is listed as hand-edited but matches the default layout"
            );
            return;
        }
        assert_eq!(
            stored, derived,
            "{name}: stored key ranges disagree with the ones its root keys {roots:?} imply"
        );
    }

    // -----------------------------------------------------------------------
    // coverage — the floors
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

    fn coverage(root: &Path) -> Vec<Trial> {
        let mut trials = Vec::new();

        for &(name, dir, ext, floor) in FLOORS {
            let at = root.join(dir);
            trials.push(aggregate(name, move || {
                let found = files_with(&at, ext).len();
                assert!(
                    found >= floor,
                    "{} holds {found} .{ext} specimens, expected at least {floor} — is \
                     the corpus present?",
                    at.display(),
                );
            }));
        }

        // The bass manual is preset 1 of b3+bass only, and it is the one organ field with
        // no home in the nine-nibble block, so its specimen count is worth its own floor.
        let organ = root.join("programs/organ");
        trials.push(aggregate("programs/organ/b3_bass", move || {
            let found = files_with(&organ, "ne5p")
                .iter()
                .filter(|p| is_b3_bass_preset1(&name_of(p)))
                .count();
            assert!(
                found >= 4,
                "expected several b3+bass preset-1 specimens, saw {found}"
            );
        }));

        let piano = root.join("programs/piano");
        trials.push(aggregate("piano_ids", move || {
            let covered: BTreeSet<(PianoCategory, u8)> = files_with(&piano, "ne5p")
                .iter()
                .map(|p| {
                    let panel = read_program(p).schema.piano_panel;
                    (panel.category, panel.piano_model.as_u8())
                })
                .collect();
            assert_eq!(
                covered.len(),
                PIANO_IDS.len(),
                "the golden table lists slots the corpus no longer covers",
            );
        }));

        let sample = root.join("programs/sample");
        trials.push(aggregate("sample_ids", move || {
            let covered: BTreeSet<u8> = files_with(&sample, "ne5p")
                .iter()
                .filter(|p| !is_skipped(p))
                .map(|p| read_program(p).schema.sample_panel.number)
                .collect();
            assert_eq!(
                covered.len(),
                SAMPLE_IDS.len(),
                "the golden table lists samples the corpus no longer covers",
            );
        }));

        let settings = root.join("settings");
        trials.push(aggregate("settings_oracle", move || {
            let present: BTreeSet<String> = files_with(&settings, "ne5s")
                .iter()
                .map(|p| oracle_key(&stem(p)))
                .collect();
            let listed: BTreeSet<String> = SETTINGS_ORACLE
                .iter()
                .map(|(s, ..)| oracle_key(s))
                .chain(SETTINGS_UNMOVED.iter().map(|(s, _)| s.to_string()))
                .chain(SELECTION_ORACLE.iter().map(|(s, ..)| s.to_string()))
                .collect();
            let stale: Vec<_> = listed.difference(&present).collect();
            assert!(
                stale.is_empty(),
                "the oracle lists specimens the corpus no longer holds: {stale:?}",
            );

            let duplicated = SETTINGS_ORACLE.len()
                - SETTINGS_ORACLE
                    .iter()
                    .map(|(s, ..)| oracle_key(s))
                    .collect::<BTreeSet<_>>()
                    .len();
            assert_eq!(duplicated, 0, "a specimen is listed twice");
        }));

        let samples = root.join("samples");
        trials.push(aggregate("nsmp_strokes", move || {
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
}

fn main() {
    #[cfg(feature = "corpus")]
    cases::run();
}
