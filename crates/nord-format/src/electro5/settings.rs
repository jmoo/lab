//! The Electro 5 global settings format (`.ne5s`).
//!
//! Reads top-down: the format's constants, then [`Schema`] — the file as `binrw` sees
//! it — then [`Settings`]. The body is one `#[bitpanel]`, in [`panel`].

pub mod panel;

pub use panel::{
    B3TrigMode, CtrlPedalGain, CtrlPedalType, FineTune, GlobalTranspose, KeyClickLevel, LiveSlot,
    Menu, MidiChannel, MidiMessageMode, OutputRouting, PercDecay, PercVolume, ResonanceLevel,
    RotaryBalance, RotaryCtrlType, RotaryPedalMode, RotaryRate, RotarySpeakerType, Selection,
    Setting, SettingsPanel, SustainPedalMode, SustainPedalType, TonewheelMode, TransposeAt,
};

use crate::common::container;
use crate::electro5::program;
use crate::error::Error;
use crate::panel::{FieldError, Panel};
use crate::types::RangedU16Pair;
use binrw::{binrw, BinRead, BinWriterExt};
use panel::BODY_LEN;
use std::fmt::Debug;
use std::io::{Cursor, Read, Seek, Write};

pub const FORMAT: &str = "ne5s";
/// Schema versions validated against the corpus. Every corpus settings file reports 0.
pub const KNOWN_VERSIONS: &[u32] = &[0];
/// What a newly authored settings file is stamped with. Reading one overwrites it with
/// whatever the file carried.
pub const DEFAULT_VERSION: u32 = 0;
/// Total file length: 44-byte CBIN header + 34-byte body.
pub const FILE_LEN: usize = container::HEADER_LEN + BODY_LEN;

pub type Location = RangedU16Pair<0, 0>;

/// The 34-byte settings body.
///
/// ⚠️ The offsets below are absolute in a **type-1 file**, as everywhere in this crate;
/// the body itself starts at [`container::HEADER_LEN`].
#[binrw]
#[derive(Debug)]
#[brw(little)]
pub struct Schema {
    // 0x2c..0x4d. Two panels share these bytes: the menu settings and the instrument's
    // selection state, interleaved rather than split (the set list song runs 30..=37 and
    // the first setting starts at 38), so the body is read once and decoded twice.
    #[brw(big)]
    #[br(temp)]
    #[bw(calc = panel::encode(panel, selection))]
    body: [u8; BODY_LEN],

    #[br(try_calc = SettingsPanel::try_from(body))]
    #[bw(ignore)]
    pub panel: SettingsPanel,

    #[br(try_calc = Selection::try_from(body))]
    #[bw(ignore)]
    pub selection: Selection,
}

impl Schema {
    /// Every settable field, menu panel then selection, in declaration order.
    pub fn fields(&self) -> Vec<program::Field> {
        let mut out = program::describe("panel", &self.panel);
        out.extend(program::describe("selection", &self.selection));
        out
    }

    /// Set one field, addressed as `panel.field` or `selection.field`.
    pub fn set_field(&mut self, path: &str, value: &str) -> Result<(), FieldError> {
        let (member, field) = path
            .split_once('.')
            .ok_or_else(|| FieldError::UnknownField {
                panel: "settings",
                name: path.to_string(),
            })?;
        match member {
            "panel" => self.panel.set_field(field, value),
            "selection" => self.selection.set_field(field, value),
            other => Err(FieldError::UnknownField {
                panel: "settings",
                name: other.to_string(),
            }),
        }
    }
}

/// Electro 5 global settings (`ne5s`): the System, MIDI and Sound menus.
///
/// The whole file is one panel plus its CBIN header; there is no slot to speak of, since
/// the instrument holds exactly one of these.
#[derive(Debug)]
pub struct Settings {
    pub schema: Schema,
    /// The container this file arrived in — see [`program::Program`].
    header: container::Header,
}

impl Settings {
    pub fn new() -> Settings {
        Settings {
            schema: Schema {
                panel: SettingsPanel::default(),
                selection: Selection::default(),
            },
            header: container::Header::new(FORMAT, DEFAULT_VERSION),
        }
    }

    pub fn read_from(reader: &mut (impl Read + Seek)) -> Result<Settings, Error> {
        let (header, _, body) =
            container::Container::open_fixed::<Location>(reader, FORMAT, KNOWN_VERSIONS, FILE_LEN)?;
        Ok(Settings {
            schema: Schema::read_be(&mut Cursor::new(body))?,
            header,
        })
    }

    pub fn write_to(&self, writer: &mut (impl Write + Seek)) -> Result<(), Error> {
        let mut body = Cursor::new(Vec::new());
        body.write_be(&self.schema)?;
        container::Container {
            header: self.header.clone(),
            location: container::location_of(0, 0),
            body: body.into_inner(),
        }
        .write_to(writer)
    }

    /// The settings body as stored, `0x2c..=0x4d`. Includes the bits no field claims.
    pub fn body(&self) -> [u8; BODY_LEN] {
        panel::encode(&self.schema.panel, &self.schema.selection)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ParseError;
    use std::io::Cursor;

    /// A settings file writes out at its declared length and reads back.
    ///
    /// The CRC region ends at the last byte of the file, and `CrcWriter` only flushes
    /// its buffered body after the write position *passes* the region — so an
    /// off-by-one in the region bound silently truncates the output to the bytes ahead
    /// of the checksum. This is the only ne5 format whose body runs to EOF, which is
    /// why it needs its own writer test.
    #[test]
    fn a_settings_file_round_trips_at_full_length() {
        let settings = Settings::new();
        let mut bytes = Vec::new();
        settings.write_to(&mut Cursor::new(&mut bytes)).unwrap();
        assert_eq!(bytes.len(), FILE_LEN);

        let back = Settings::read_from(&mut Cursor::new(&mut bytes)).unwrap();
        assert_eq!(back.body(), settings.body());
    }

    /// Bits no field claims are kept, so a body the decoder accepts survives verbatim.
    #[test]
    fn the_unclaimed_bits_survive_a_round_trip() {
        let mut settings = Settings::new();
        // Bit 18 is the only bit inside the decoded run that belongs to no field — the
        // third bit of `0x2e` — and `0x3e` onwards is past the last field, which ends at
        // bit 141. Both have to come back untouched.
        let mut body = settings.body();
        body[0x2e - 0x2c] = 0x20;
        body[0x3e - 0x2c] = 0x5a;
        settings.schema.panel = SettingsPanel::try_from(body).unwrap();
        settings.schema.selection = Selection::try_from(body).unwrap();

        let mut bytes = Vec::new();
        settings.write_to(&mut Cursor::new(&mut bytes)).unwrap();
        assert_eq!(bytes.len(), FILE_LEN);

        let back = Settings::read_from(&mut Cursor::new(&mut bytes)).unwrap();
        assert_eq!(back.body(), body);
    }

    /// Both panels share the body, so editing one must not revert the other.
    ///
    /// Each carries a stale copy of the other's fields in the bytes it was decoded from;
    /// this is what catches an encode that keeps only one panel's output instead of
    /// threading it through the other's.
    #[test]
    fn the_two_panels_do_not_overwrite_each_other() {
        let mut settings = Settings::new();
        settings.schema.panel.b3_tonewheel_mode = TonewheelMode::Vintage3;
        settings.schema.selection.live_mode = true;
        settings.schema.selection.program = (4, 2).try_into().unwrap();

        let mut bytes = Vec::new();
        settings.write_to(&mut Cursor::new(&mut bytes)).unwrap();
        let back = Settings::read_from(&mut Cursor::new(&mut bytes)).unwrap();

        assert_eq!(back.schema.panel.b3_tonewheel_mode, TonewheelMode::Vintage3);
        assert!(back.schema.selection.live_mode);
        assert_eq!(back.schema.selection.program.inner(), (4, 2));
    }

    /// Fields are addressed the way `--set` spells them, and both members answer.
    #[test]
    fn fields_are_set_through_schema_paths() {
        let mut settings = Settings::new();
        settings
            .schema
            .set_field("panel.global_transpose", "+3")
            .unwrap();
        settings
            .schema
            .set_field("selection.live_mode", "on")
            .unwrap();

        let mut bytes = Vec::new();
        settings.write_to(&mut Cursor::new(&mut bytes)).unwrap();
        let back = Settings::read_from(&mut Cursor::new(&mut bytes)).unwrap();
        assert_eq!(back.schema.panel.global_transpose.inner(), 3);
        assert!(back.schema.selection.live_mode);

        let paths: Vec<String> = settings
            .schema
            .fields()
            .into_iter()
            .map(|f| f.path)
            .collect();
        assert!(paths.contains(&"panel.global_transpose".to_string()));
        assert!(paths.contains(&"selection.program".to_string()));

        assert!(settings.schema.set_field("panel.no_such", "1").is_err());
        assert!(settings.schema.set_field("global_transpose", "1").is_err());
    }

    /// An unknown schema version is refused at read rather than decoded on a guess: a
    /// firmware that bumps it could move every field below.
    #[test]
    fn an_unknown_schema_version_is_refused() {
        let settings = Settings::new();
        let mut bytes = Vec::new();
        settings.write_to(&mut Cursor::new(&mut bytes)).unwrap();
        assert!(Settings::read_from(&mut Cursor::new(&mut bytes.clone())).is_ok());

        // The schema version lives at 0x14, little-endian.
        bytes[0x14..0x18].copy_from_slice(&1u32.to_le_bytes());
        let err = Settings::read_from(&mut Cursor::new(&mut bytes))
            .expect_err("version 1 must not decode");
        assert!(
            matches!(
                err,
                Error::Parse(ParseError::UnsupportedVersion {
                    format: FORMAT,
                    version: 1,
                    ..
                })
            ),
            "unhelpful error: {err}",
        );
    }
}
