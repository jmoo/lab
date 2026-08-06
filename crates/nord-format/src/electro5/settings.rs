//! The Electro 5 global settings format (`.ne5s`).
//!
//! Reads top-down: the format's constants, then [`SettingsBody`] — the 34 bytes
//! after the container header, one flat `#[bitbody]` in [`panel`] — then
//! [`Settings`].

pub mod panel;

pub use panel::{
    B3TrigMode, CtrlPedalGain, CtrlPedalType, FineTune, GlobalTranspose, KeyClickLevel, LiveSlot,
    Menu, MidiChannel, MidiMessageMode, OutputRouting, PercDecay, PercVolume, ResonanceLevel,
    RotaryBalance, RotaryCtrlType, RotaryPedalMode, RotaryRate, RotarySpeakerType, Setting,
    SettingsBody, SustainPedalMode, SustainPedalType, TonewheelMode, TransposeAt,
};

use crate::cbin::{self, Cbin, Header};
use crate::electro5::program;
use crate::error::Error;
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
                body: SettingsBody::default(),
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
