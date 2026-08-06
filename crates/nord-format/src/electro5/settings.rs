//! The Electro 5 global settings format (`.ne5s`).
//!
//! Reads top-down: the format's constants, then [`SettingsBody`] — the 34 bytes
//! after the container header — then [`Settings`]. The body is one `#[bitpanel]`,
//! in [`panel`].

pub mod panel;

pub use panel::{
    B3TrigMode, CtrlPedalGain, CtrlPedalType, FineTune, GlobalTranspose, KeyClickLevel, LiveSlot,
    Menu, MidiChannel, MidiMessageMode, OutputRouting, PercDecay, PercVolume, ResonanceLevel,
    RotaryBalance, RotaryCtrlType, RotaryPedalMode, RotaryRate, RotarySpeakerType, Selection,
    Setting, SettingsPanel, SustainPedalMode, SustainPedalType, TonewheelMode, TransposeAt,
};

use crate::cbin::{self, BodyReader, BodyWriter, Cbin, Header};
use crate::electro5::program;
use crate::error::Error;
use crate::panel::{FieldError, Panel};
use panel::BODY_LEN;
use std::fmt::Debug;
use std::io::{Read, Seek, Write};

pub const FORMAT: &str = "ne5s";
/// Schema versions validated against the corpus. Every corpus settings file reports 0.
pub const KNOWN_VERSIONS: &[u32] = &[0];
/// Type-1 file length: 44-byte CBIN header + 34-byte body.
pub const FILE_LEN: usize = 0x2c + BODY_LEN;

/// The settings as a file: container header plus body.
pub type Schema = Cbin<SettingsBody>;

/// The 34-byte settings body. Two panels share these bytes: the menu settings and
/// the instrument's selection state, interleaved rather than split (the set list
/// song runs 30..=37 and the first setting starts at 38), so the body is read once
/// and decoded twice — an overlay `#[bitbody]`'s one-segment-per-byte model does
/// not express, which is why this codec is by hand.
#[derive(Debug)]
pub struct SettingsBody {
    pub panel: SettingsPanel,
    pub selection: Selection,
}

impl cbin::Body for SettingsBody {
    const LEN: Option<u64> = Some(BODY_LEN as u64);

    fn read<R: Read + Seek>(r: &mut BodyReader<'_, R>, _: &Header) -> Result<Self, Error> {
        let mut raw = [0u8; BODY_LEN];
        r.read_exact(&mut raw)?;
        Ok(SettingsBody {
            panel: SettingsPanel::try_from(raw)?,
            selection: Selection::try_from(raw)?,
        })
    }

    fn write<W: Write + Seek>(&self, w: &mut BodyWriter<'_, W>) -> Result<(), Error> {
        w.write_all(&panel::encode(&self.panel, &self.selection))?;
        Ok(())
    }
}

impl SettingsBody {
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
}

impl Settings {
    pub fn new() -> Settings {
        Settings {
            schema: Cbin {
                header: Header::new(FORMAT, (0, 0), 0),
                body: SettingsBody {
                    panel: SettingsPanel::default(),
                    selection: Selection::default(),
                },
            },
        }
    }

    pub fn read_from(reader: &mut (impl Read + Seek)) -> Result<Settings, Error> {
        let schema: Schema = cbin::read(reader, FORMAT)?;
        program::known_version(FORMAT, schema.header.version, KNOWN_VERSIONS)?;
        Ok(Settings { schema })
    }

    pub fn write_to(&self, writer: &mut (impl Write + Seek)) -> Result<(), Error> {
        self.schema.write_to(writer)
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
    use std::io::Cursor;

    #[test]
    fn settings_round_trip_at_the_declared_length() {
        let settings = Settings::new();
        let mut bytes = Vec::new();
        settings.write_to(&mut Cursor::new(&mut bytes)).unwrap();
        assert_eq!(bytes.len(), FILE_LEN);
        assert_eq!(&bytes[0x08..0x0c], FORMAT.as_bytes());

        let back = Settings::read_from(&mut Cursor::new(&mut bytes)).unwrap();
        let mut again = Vec::new();
        back.write_to(&mut Cursor::new(&mut again)).unwrap();
        assert_eq!(bytes, again);
    }

    #[test]
    fn a_settings_field_set_by_name_survives_a_round_trip() {
        let mut settings = Settings::new();
        settings
            .schema
            .set_field("panel.global_transpose", "-3")
            .unwrap();

        let mut bytes = Vec::new();
        settings.write_to(&mut Cursor::new(&mut bytes)).unwrap();
        let back = Settings::read_from(&mut Cursor::new(&mut bytes)).unwrap();
        let listed = back
            .schema
            .fields()
            .into_iter()
            .find(|f| f.path == "panel.global_transpose")
            .expect("declared");
        assert_eq!(listed.display, "-3");
    }
}
