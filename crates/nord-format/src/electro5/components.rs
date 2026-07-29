//! Typed components for the Electro 5's panel values.
//!
//! RFC-0001's third goal: a reusable unit is *a typed field that knows how to read, write
//! and display itself*, reused across models rather than bound to one panel's offset.
//! These are those units. Each one implements [`Packed`], so it drops into a
//! `#[bits(...)]` placement anywhere the same value appears — and each owns its own
//! `Display`, so presentation lives with the value instead of in a lookup table on the
//! far side of a crate boundary.
//!
//! # How unknown values are handled, and why not by refusing them
//!
//! Two shapes appear here, and the difference is not a style choice:
//!
//! * **Total** values, where every bit pattern the slot can hold is meaningful — a
//!   [`Level`] is any of 128 knob positions, a [`Routing`] is any of four two-bit states.
//!   Decoding cannot fail, and there is nothing to be unknown about.
//! * **Sparse** values, where the slot is wider than the set of meanings we have
//!   recovered — [`Fx1Type`] has eight known types in a four-bit field. These carry an
//!   `Unknown(u8)` variant.
//!
//! The tempting alternative is to refuse a value we cannot name, on the same reasoning as
//! [`crate::error::ParseError::UnsupportedVersion`]: better to fail than to decode a guess
//! and write it back. That reasoning does not carry over, and the corpus says so
//! plainly — **604 of 617 programs** hold an effect routing of `0`, a value with no
//! recovered meaning. A decoder that refused the unrecognised would reject 98% of the
//! only evidence we have.
//!
//! The difference is what refusal protects against. An unknown *schema version* means
//! every field offset is suspect, so nothing can be trusted. An unknown *value in a known
//! field* is one value: its position is certain, its round-trip is exact, and the only
//! thing missing is a name. Refusing costs the whole file to gain nothing.
//!
//! So the rule here is **preserve, and make it visible**: decode succeeds, the bits
//! round-trip byte-exactly, `Display` says `unknown (9)` rather than inventing a label,
//! and [`Unknown`](Fx1Type::Unknown) is a variant a caller can match on — which means a
//! corpus sweep can *report* unrecognised values instead of hiding them. That turns a
//! reverse-engineering gap into a work item. Silently coercing it to the nearest known
//! label is the one option that is actually unsafe, and it is what the pre-component code
//! did: an effect routing of `0` and one of `1` both printed `off`.

use std::fmt::{self, Debug, Display, Formatter};

use crate::bits::{bits_for, Packed};
use crate::error::ParseError;

/// A 0..=127 continuous control — rate, level, compression, gain, tone.
///
/// The instrument shows most of these as 0..10 on the panel, which is a pure display
/// transform over the stored byte and so lives here rather than in whatever is rendering.
/// Total over seven bits: every value is a real position.
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
    /// `96 (7.6)` — the stored byte and the panel reading, which is how the value is
    /// useful when comparing a file against the instrument in front of you.
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
/// Stored **one higher than the panel position**: the `programs/fx` specimens are named
/// `fx1_1xx` for lower and `fx1_2xx` for upper, and they decode to 2 and 3.
///
/// [`Unset`](Self::Unset) is the fourth two-bit state, and it is the common one — 604 of
/// 617 corpus programs carry it on at least one effect. Its meaning has not been
/// recovered; no specimen was ever captured with the panel's *off* position, so whether
/// `0` and `1` differ audibly is an open question. It is named rather than folded into
/// `Off` so that the two stay distinguishable, because the pre-component code printed
/// both as `off` and lost the distinction.
///
/// Total: two bits, four states, no `Unknown` needed.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Routing {
    /// Stored `0`. Meaning not recovered; see the type docs.
    #[default]
    Unset,
    /// Stored `1` — the panel's *off*.
    Off,
    /// Stored `2`.
    Lower,
    /// Stored `3`.
    Upper,
}

impl Routing {
    /// The routing selected by the panel's own numbering — `0` off, `1` lower, `2`
    /// upper — which is what the `programs/fx` specimen filenames record. The stored
    /// value is one higher; keeping that relationship here means a caller never has to
    /// remember which numbering it is holding.
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
            Routing::Unset | Routing::Off => None,
        }
    }
}

impl Packed for Routing {
    const MAX_BITS: u32 = 2;
    type Error = std::convert::Infallible;

    fn from_bits(bits: u64) -> Result<Self, Self::Error> {
        Ok(match bits & 0b11 {
            0 => Routing::Unset,
            1 => Routing::Off,
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
        match self.part() {
            Some(part) => f.write_str(part),
            None => f.write_str("off"),
        }
    }
}

/// Declare a sparse enumeration: a closed set of recovered meanings inside a slot that
/// can hold more, plus the `Unknown` variant that keeps the rest readable.
macro_rules! sparse_enum {
    (
        $(#[$meta:meta])*
        $name:ident, $bits:expr, { $($value:expr => $variant:ident, $label:expr;)+ }
    ) => {
        $(#[$meta])*
        ///
        /// Sparse: the slot holds more values than we have names for, so an unrecognised
        /// one decodes to `Unknown` and round-trips byte-exactly rather than being
        /// refused or coerced. See the module docs.
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $($variant,)+
            /// A value in this field that no specimen has explained yet.
            Unknown(u8),
        }

        impl $name {
            /// The recovered label, or `None` for a value we cannot name.
            pub fn label(&self) -> Option<&'static str> {
                match self {
                    $($name::$variant => Some($label),)+
                    $name::Unknown(_) => None,
                }
            }

            /// Whether this is a value no specimen has explained. A corpus sweep can use
            /// this to *report* reverse-engineering gaps rather than hide them.
            pub fn is_unknown(&self) -> bool {
                matches!(self, $name::Unknown(_))
            }

            /// The stored value, named or not — so a gap can be reported precisely
            /// without the caller importing [`Packed`].
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
    /// Effect 1's modulation type. Four bits; eight recovered.
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
    /// Effect 2's modulation type. Four bits; six recovered.
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
    /// The speaker / amp simulation. Three bits; six recovered.
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
    /// The reverb algorithm. Three bits; five recovered.
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
    /// **b3+bass is a selection, not a fifth model.** It shares the B3's storage, but its
    /// two presets are different instruments: preset 1 is the bass manual, where only
    /// drawbars 1–2 do anything and they live outside the nine-nibble block (see
    /// [`crate::electro5::program::OrganPanel::b3_bass_drawbars`]); preset 2 is an
    /// ordinary B3.
    OrganType, 3, {
        0 => B3, "b3";
        1 => B3Bass, "b3+bass";
        2 => Pipe, "pipe";
        3 => Vox, "vox";
        4 => Farfisa, "farfisa";
    }
);

sparse_enum!(
    /// Which part the equalizer applies to.
    ///
    /// Whether the EQ is engaged at all is a separate bit, so `Lower` means *lower*, not
    /// *off*.
    EqualizerPart, 2, {
        0 => Lower, "lower";
        1 => Upper, "upper";
        2 => Both, "lower+upper";
    }
);

sparse_enum!(
    /// The piano panel's Type dial — which library category the model is drawn from.
    /// Labels are the backup bundle's own directory names.
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
    /// Which model's storage slots this selection reads from.
    ///
    /// b3+bass is not a fifth model — it shares the B3's slots — so this collapses the
    /// two onto [`OrganModel::B3`]. `None` for a selection we cannot name.
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

    /// Whether preset 1 is the bass manual rather than an ordinary registration. In that
    /// mode the nine-nibble block holds stale values and
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

    /// The whole point of the sparse shape: a value we cannot name still reads, still
    /// writes back the same bits, and says so rather than borrowing a label.
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

    /// `Routing` is total over its two bits, so every state has a name — including the
    /// one whose meaning is still open.
    #[test]
    fn routing_is_total() {
        assert_eq!(Routing::from_bits(0).unwrap(), Routing::Unset);
        assert_eq!(Routing::from_bits(1).unwrap(), Routing::Off);
        assert_eq!(Routing::from_bits(2).unwrap(), Routing::Lower);
        assert_eq!(Routing::from_bits(3).unwrap(), Routing::Upper);

        for bits in 0..4u64 {
            assert_eq!(Routing::from_bits(bits).unwrap().to_bits(), bits);
        }

        // Unset and Off both read as "not routed", but they are different bytes and the
        // type keeps them apart.
        assert_eq!(Routing::Unset.part(), None);
        assert_eq!(Routing::Off.part(), None);
        assert_ne!(Routing::Unset, Routing::Off);
    }
}
