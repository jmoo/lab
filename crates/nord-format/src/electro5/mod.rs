pub mod live;
pub mod settings;
pub use settings::Settings;
pub mod song;
pub use song::Song;
pub mod program;
pub use program::{OrganModel, PercSpeed, Program, VibChorus};
#[cfg(feature = "bundle")]
pub mod bundle;
use crate::common;
#[cfg(feature = "bundle")]
pub use bundle::Bundle;

pub type OctaveShift = common::OctaveShift<7, -6, 6>;
pub type Transpose = common::Transpose<6, -6, 6>;
pub type SplitPoint = common::SplitPoint73;
pub type PartMix = common::PartMix;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum Instrument {
    #[default]
    Organ,
    Piano,
    Sample,
}

impl Instrument {
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    pub fn as_u16(&self) -> u16 {
        *self as u16
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Instrument::Organ => "organ",
            Instrument::Piano => "piano",
            Instrument::Sample => "sample",
        }
    }
}

impl crate::bits::Packed for Instrument {
    // Three variants, so two bits — even though the panel's slot is three bits wide.
    const MAX_BITS: u32 = 2;
    type Error = crate::error::ParseError;

    fn from_bits(bits: u64) -> Result<Self, Self::Error> {
        Instrument::try_from(bits as u8)
            .map_err(|e| crate::error::ParseError::OutOfBounds(format!("{bits}"), e.to_string()))
    }

    fn to_bits(&self) -> u64 {
        self.as_u8() as u64
    }
}

impl TryFrom<u8> for Instrument {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Instrument, Self::Error> {
        match value {
            0 => Ok(Instrument::Organ),
            1 => Ok(Instrument::Piano),
            2 => Ok(Instrument::Sample),
            _ => Err("Value is out of range for instrument"),
        }
    }
}
