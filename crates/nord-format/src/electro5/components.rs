//! Typed values for the Electro 5's panels.
//!
//! Each implements [`Packed`], so it drops into any `#[bits(...)]` placement holding that
//! value, and owns its own `Display`.
//!
//! Some are **total** — every bit pattern the slot can hold is meaningful, so decoding
//! cannot fail. The rest are **sparse**: the slot is wider than the set of values we have
//! names for, and anything unrecognised decodes to `Unknown`, round-trips unchanged, and
//! displays as `unknown (9)`. Match on it, or call `is_unknown()`, to find them; nothing
//! is silently relabelled.

use std::fmt::{self, Debug, Display, Formatter};

use crate::bits::{bits_for, Packed};
use crate::error::ParseError;

/// A 0..=127 continuous control — rate, level, compression, gain, tone. The panel shows
/// most of these as 0..10.
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Level {
    inner: u8,
}

impl Level {
    pub const MAX: u8 = 127;

    pub fn new(value: u8) -> Result<Self, ParseError> {
        value.try_into()
    }

    /// The stored value, 0..=127.
    pub fn as_u8(&self) -> u8 {
        self.inner
    }

    /// The panel's 0..10 reading.
    pub fn as_panel(&self) -> f32 {
        f32::from(self.inner) / 127.0 * 10.0
    }
}

impl TryFrom<u8> for Level {
    type Error = ParseError;

    fn try_from(value: u8) -> Result<Self, ParseError> {
        if value > Self::MAX {
            return Err(ParseError::OutOfBounds(
                format!("{value}"),
                format!("{}", Self::MAX),
            ));
        }
        Ok(Level { inner: value })
    }
}

impl Packed for Level {
    const MAX_BITS: u32 = bits_for(Level::MAX as u64);
    type Error = ParseError;

    fn from_bits(bits: u64) -> Result<Self, ParseError> {
        (bits as u8).try_into()
    }

    fn to_bits(&self) -> u64 {
        self.inner as u64
    }
}

impl Display for Level {
    /// Stored byte and panel reading: `96 (7.6)`.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:.1})", self.inner, self.as_panel())
    }
}

impl Debug for Level {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl PartialEq<u8> for Level {
    fn eq(&self, other: &u8) -> bool {
        self.inner == *other
    }
}

/// Which part an effect is routed to.
///
/// The stored encoding is not the panel's numbering: off agrees at `0`, but the two
/// engaged positions are `2` and `3`.
///
/// | stored | 0 | 1 | 2 | 3 |
/// |---|---|---|---|---|
/// | | off | [`Unknown`](Self::Unknown) | lower | upper |
///
/// Total over two bits, so decoding cannot fail.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Routing {
    #[default]
    Off = 0,
    /// Appears in real programs but has no known meaning, and the fx selector cannot
    /// produce it.
    Unknown = 1,
    Lower = 2,
    Upper = 3,
}

impl Routing {
    /// The routing at a panel position: `0` off, `1` lower, `2` upper.
    pub fn from_panel(position: u8) -> Option<Routing> {
        match position {
            0 => Some(Routing::Off),
            1 => Some(Routing::Lower),
            2 => Some(Routing::Upper),
            _ => None,
        }
    }

    /// Which part the effect actually reaches, or `None` when it is not engaged.
    pub fn part(&self) -> Option<&'static str> {
        match self {
            Routing::Lower => Some("lower"),
            Routing::Upper => Some("upper"),
            Routing::Off | Routing::Unknown => None,
        }
    }

    /// Whether this is the value with no known meaning. Unlike the sparse enumerations,
    /// this one does occur in practice.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Routing::Unknown)
    }
}

impl Packed for Routing {
    const MAX_BITS: u32 = 2;
    type Error = std::convert::Infallible;

    fn from_bits(bits: u64) -> Result<Self, Self::Error> {
        Ok(match bits & 0b11 {
            0 => Routing::Off,
            1 => Routing::Unknown,
            2 => Routing::Lower,
            _ => Routing::Upper,
        })
    }

    fn to_bits(&self) -> u64 {
        *self as u64
    }
}

impl Display for Routing {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Routing::Off => f.write_str("off"),
            Routing::Unknown => f.write_str("unknown (1)"),
            Routing::Lower => f.write_str("lower"),
            Routing::Upper => f.write_str("upper"),
        }
    }
}

/// Declare a sparse enumeration: known values, plus `Unknown` for the rest of the slot.
macro_rules! sparse_enum {
    (
        $(#[$meta:meta])*
        $name:ident, $bits:expr, { $($value:expr => $variant:ident, $label:expr;)+ }
    ) => {
        $(#[$meta])*
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $($variant,)+
            /// A stored value with no known meaning.
            Unknown(u8),
        }

        impl $name {
            /// The label, or `None` for a value with no known meaning.
            pub fn label(&self) -> Option<&'static str> {
                match self {
                    $($name::$variant => Some($label),)+
                    $name::Unknown(_) => None,
                }
            }

            /// Whether the stored value has no known meaning.
            pub fn is_unknown(&self) -> bool {
                matches!(self, $name::Unknown(_))
            }

            /// The stored value, named or not.
            pub fn raw(&self) -> u8 {
                <Self as Packed>::to_bits(self) as u8
            }
        }

        impl Default for $name {
            fn default() -> Self {
                $name::from_bits(0).expect("decoding is total")
            }
        }

        impl Packed for $name {
            const MAX_BITS: u32 = $bits;
            type Error = std::convert::Infallible;

            fn from_bits(bits: u64) -> Result<Self, Self::Error> {
                Ok(match bits as u8 {
                    $($value => $name::$variant,)+
                    other => $name::Unknown(other),
                })
            }

            fn to_bits(&self) -> u64 {
                match self {
                    $($name::$variant => $value as u64,)+
                    $name::Unknown(raw) => *raw as u64,
                }
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                match self.label() {
                    Some(label) => f.write_str(label),
                    None => write!(f, "unknown ({})", self.to_bits()),
                }
            }
        }
    };
}

sparse_enum!(
    /// Effect 1's modulation type.
    Fx1Type, 4, {
        0 => Trem1, "trem 1";
        1 => Trem2, "trem 2";
        2 => Trem1And2, "trem 1&2";
        3 => Pan1, "pan 1";
        4 => Pan2, "pan 2";
        5 => Pan1And2, "pan 1&2";
        6 => Wah, "wah";
        7 => Rm, "rm";
    }
);

sparse_enum!(
    /// Effect 2's modulation type.
    Fx2Type, 4, {
        0 => Phaser1, "phaser 1";
        1 => Phaser2, "phaser 2";
        2 => Flanger, "flanger";
        3 => Chorus1, "chorus 1";
        4 => Chorus2, "chorus 2";
        5 => Vibe, "vibe";
    }
);

sparse_enum!(
    /// The speaker / amp simulation.
    Fx3Type, 3, {
        0 => None_, "none";
        1 => Small, "small";
        2 => Jc, "jc";
        3 => Twin, "twin";
        4 => Rotary, "rotary";
        5 => Comp, "comp";
    }
);

sparse_enum!(
    /// The reverb algorithm.
    Fx5Type, 3, {
        0 => Room, "room";
        1 => StageSoft, "stage soft";
        2 => Stage, "stage";
        3 => HallSoft, "hall soft";
        4 => Hall, "hall";
    }
);

sparse_enum!(
    /// Which organ the program has selected.
    ///
    /// b3+bass shares the B3's storage rather than being a fifth model, and its preset 1
    /// is the bass manual — see [`Self::is_b3_bass`].
    OrganType, 3, {
        0 => B3, "b3";
        1 => B3Bass, "b3+bass";
        2 => Pipe, "pipe";
        3 => Vox, "vox";
        4 => Farfisa, "farfisa";
    }
);

sparse_enum!(
    /// Which part the equalizer applies to. Whether it is engaged is a separate bit, so
    /// `Lower` means lower, not off.
    EqualizerPart, 2, {
        0 => Lower, "lower";
        1 => Upper, "upper";
        2 => Both, "lower+upper";
    }
);

sparse_enum!(
    /// The piano panel's Type dial — which library category the model comes from.
    PianoCategory, 3, {
        0 => Grand, "grand";
        1 => Upright, "upright";
        2 => EPiano1, "epiano1";
        3 => EPiano2, "epiano2";
        4 => Clavinet, "clavinet";
        5 => Harpsichord, "harps";
    }
);

impl OrganType {
    /// Which model's storage slots this selection reads from. b3 and b3+bass share
    /// [`OrganModel::B3`]; `None` for an unknown selection.
    pub fn storage(&self) -> Option<crate::electro5::program::OrganModel> {
        use crate::electro5::program::OrganModel;
        match self {
            OrganType::B3 | OrganType::B3Bass => Some(OrganModel::B3),
            OrganType::Pipe => Some(OrganModel::Pipe),
            OrganType::Vox => Some(OrganModel::Vox),
            OrganType::Farfisa => Some(OrganModel::Farfisa),
            OrganType::Unknown(_) => None,
        }
    }

    /// Whether preset 1 is the bass manual. In that mode the nine-nibble block holds
    /// stale values and
    /// [`OrganPanel::b3_bass_drawbars`](crate::electro5::program::OrganPanel::b3_bass_drawbars)
    /// is the only correct source for bars 1-2.
    pub fn is_b3_bass(&self) -> bool {
        matches!(self, OrganType::B3Bass)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_carries_the_panel_transform() {
        assert_eq!(Level::new(0).unwrap().to_string(), "0 (0.0)");
        assert_eq!(Level::new(127).unwrap().to_string(), "127 (10.0)");
        assert_eq!(Level::new(96).unwrap().to_string(), "96 (7.6)");
        assert!(Level::new(128).is_err(), "128 does not fit seven bits");
    }

    /// An unrecognised value reads, writes back the same bits, and says so.
    #[test]
    fn an_unrecognised_value_survives_and_announces_itself() {
        let unknown = Fx1Type::from_bits(9).unwrap();
        assert_eq!(unknown, Fx1Type::Unknown(9));
        assert!(unknown.is_unknown());
        assert_eq!(unknown.label(), None);
        assert_eq!(unknown.to_string(), "unknown (9)");
        assert_eq!(unknown.to_bits(), 9, "an unknown value must round-trip");
    }

    /// Every recovered value round-trips, and none of them is reported as unknown.
    #[test]
    fn recovered_values_round_trip() {
        for bits in 0..8u64 {
            let t = Fx1Type::from_bits(bits).unwrap();
            assert!(!t.is_unknown(), "{bits} should be recovered");
            assert_eq!(t.to_bits(), bits);
        }
        for bits in 0..5u64 {
            assert_eq!(OrganType::from_bits(bits).unwrap().to_bits(), bits);
        }
        for bits in 0..3u64 {
            assert_eq!(EqualizerPart::from_bits(bits).unwrap().to_bits(), bits);
        }
    }

    #[test]
    fn routing_matches_what_the_instrument_stores() {
        assert_eq!(Routing::from_bits(0).unwrap(), Routing::Off);
        assert_eq!(Routing::from_bits(1).unwrap(), Routing::Unknown);
        assert_eq!(Routing::from_bits(2).unwrap(), Routing::Lower);
        assert_eq!(Routing::from_bits(3).unwrap(), Routing::Upper);

        for bits in 0..4u64 {
            assert_eq!(Routing::from_bits(bits).unwrap().to_bits(), bits);
        }

        // Off agrees at 0, but the engaged positions land on 2 and 3.
        assert_eq!(Routing::from_panel(0), Some(Routing::Off));
        assert_eq!(Routing::from_panel(1), Some(Routing::Lower));
        assert_eq!(Routing::from_panel(2), Some(Routing::Upper));
        assert_eq!(Routing::from_panel(3), None);
        assert_eq!(Routing::Lower.to_bits(), 2);
        assert_eq!(Routing::Upper.to_bits(), 3);

        // The unknown state must not render as `off`.
        assert_eq!(Routing::Off.to_string(), "off");
        assert_eq!(Routing::Unknown.to_string(), "unknown (1)");
        assert!(Routing::Unknown.is_unknown());
        assert!(!Routing::Off.is_unknown());
        assert_eq!(Routing::Unknown.part(), None);
    }
}
