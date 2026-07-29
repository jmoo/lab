//! Typed bit fields over a private backing word.
//!
//! This is the shared-component layer proposed as **option B+** in RFC-0001 (Bitfield
//! Packing). A panel keeps its packed integer *private* and exposes each logical value
//! through a [`Field`], which owns the field's position, its width, and both directions
//! of the conversion. There is no way to mutate a decoded value into the void, because
//! there is no public decoded value to mutate — only a setter that performs a
//! read-modify-write on the word.
//!
//! Three properties are worth stating explicitly, because they are the reasons this
//! layer exists:
//!
//! 1. **Position is authored once.** `Field<T, HI, LO>` names the inclusive bit range;
//!    mask, shift and width are all derived from it. The old style spelled the same
//!    position three times — as a `0b…` mask, as a `>> ((8*N)+M)` shift, and implicitly
//!    as a width — and two shipped bugs came from those three disagreeing.
//! 2. **Writes are symmetric with reads by construction.** `get` and `set` share `HI`
//!    and `LO`, so a setter cannot land somewhere its getter does not look.
//! 3. **A value cannot silently overrun its slot.** For a type whose every value
//!    provably fits (`T::MAX_BITS <= WIDTH`) [`Field::set`] is infallible and the proof
//!    is a compile-time assertion. For a raw integer in a narrower slot — a 7-bit `gain`
//!    held as `u8` — `set` does not compile and [`Field::checked_set`] must be used,
//!    which reports the overflow instead of corrupting the neighbouring field.
//!
//! Gap bits need no declaration. Whatever is not named by a `Field` stays untouched in
//! the backing word and round-trips verbatim, which is what lets a panel migrate one
//! field at a time.

use std::convert::Infallible;
use std::fmt::{self, Display, Formatter};
use std::marker::PhantomData;

/// An unsigned integer that can host packed bit fields.
///
/// Implemented for `u8`/`u16`/`u32`/`u64` — the backing words the Nord panels actually
/// use. Bits are numbered from the least-significant bit of the whole word, so a field's
/// `HI`/`LO` can be read straight off a big-endian hex dump once the word's byte span is
/// known.
pub trait Word: Copy {
    /// Width of the word in bits.
    const BITS: u32;

    fn to_u64(self) -> u64;
    fn from_u64(bits: u64) -> Self;
}

macro_rules! impl_word {
    ($($t:ty),* $(,)?) => { $(
        impl Word for $t {
            const BITS: u32 = <$t>::BITS;

            fn to_u64(self) -> u64 {
                self as u64
            }

            fn from_u64(bits: u64) -> Self {
                bits as $t
            }
        }
    )* };
}

impl_word!(u8, u16, u32, u64);

/// A value that knows how to live inside a packed bit field.
///
/// This is the reusable half of the "shared component" RFC-0001 asks for: the type owns
/// its encoding, its validation and (via `Debug`/`Display`) its presentation, and knows
/// nothing about which panel, word or offset it happens to be stored at. The same
/// `impl` serves every model that packs the same value.
pub trait Packed: Sized {
    /// The widest field this type can ever need — the number of bits its largest
    /// encoding occupies. Used to prove statically that a value fits its slot.
    const MAX_BITS: u32;

    /// Why a raw bit pattern is not a valid value of this type. [`Infallible`] for
    /// types that decode totally (`bool`, raw integers).
    type Error;

    /// Decode from the field's raw bits, already shifted down to bit 0 and masked.
    fn from_bits(bits: u64) -> Result<Self, Self::Error>;

    /// Encode to the field's raw bits, in the same shifted-down form.
    fn to_bits(&self) -> u64;
}

/// Number of bits needed to represent `max`.
pub const fn bits_for(max: u64) -> u32 {
    64 - max.leading_zeros()
}

impl Packed for bool {
    const MAX_BITS: u32 = 1;
    type Error = Infallible;

    fn from_bits(bits: u64) -> Result<Self, Infallible> {
        Ok(bits != 0)
    }

    fn to_bits(&self) -> u64 {
        *self as u64
    }
}

macro_rules! impl_packed_uint {
    ($($t:ty),* $(,)?) => { $(
        impl Packed for $t {
            const MAX_BITS: u32 = <$t>::BITS;
            type Error = Infallible;

            fn from_bits(bits: u64) -> Result<Self, Infallible> {
                Ok(bits as $t)
            }

            fn to_bits(&self) -> u64 {
                *self as u64
            }
        }
    )* };
}

impl_packed_uint!(u8, u16, u32, u64);

/// A value too wide for the field it was written to.
///
/// Nothing checked this before, because nothing wrote: panels kept their backing word
/// verbatim and decoded fields were `#[bw(ignore)]`. Once writes are derived from
/// decoded values, an over-wide value would silently overwrite the *neighbouring*
/// field's bits, so it has to be an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldOverflow {
    /// The encoded value that did not fit.
    pub value: u64,
    /// The field's width in bits.
    pub width: u32,
}

impl Display for FieldOverflow {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "value {} does not fit in a {}-bit field (max {})",
            self.value,
            self.width,
            (1u64 << self.width) - 1,
        )
    }
}

impl std::error::Error for FieldOverflow {}

/// One logical value packed into bits `LO..=HI` of a backing word.
///
/// `HI` and `LO` are inclusive and numbered from the word's least-significant bit, so
/// `Field<Instrument, 15, 13>` is the top three bits of a `u16`. The type is never
/// instantiated — it is a name for a position plus a conversion, used as
/// `MyField::get(word)` / `MyField::set(&mut word, v)`.
pub struct Field<T, const HI: u32, const LO: u32>(PhantomData<fn() -> T>);

/// Proof that a field lies inside the word it is being applied to. Checked per
/// `(word type, HI)` pair at monomorphisation, so a `Field<_, 33, 30>` read out of a
/// `u32` fails to build rather than silently returning zero.
struct WordFits<W, const HI: u32>(PhantomData<fn() -> W>);

impl<W: Word, const HI: u32> WordFits<W, HI> {
    const OK: () = assert!(HI < W::BITS, "bit field extends past its backing word");
}

impl<T: Packed, const HI: u32, const LO: u32> Field<T, HI, LO> {
    /// Width of the field in bits.
    pub const WIDTH: u32 = {
        assert!(HI >= LO, "bit field's HI must not be below its LO");
        HI - LO + 1
    };

    /// The field's mask, shifted down to bit 0.
    pub const MASK: u64 = if Self::WIDTH == 64 {
        u64::MAX
    } else {
        (1u64 << Self::WIDTH) - 1
    };

    /// Proof that every value of `T` fits this field. Forced by [`Self::set`], so a
    /// type that can overflow its slot has to go through [`Self::checked_set`].
    const FITS: () = assert!(
        T::MAX_BITS <= HI - LO + 1,
        "this type can hold values wider than the field; use `checked_set`",
    );

    /// Decode the field out of `word`.
    pub fn get<W: Word>(word: W) -> Result<T, T::Error> {
        let () = WordFits::<W, HI>::OK;
        T::from_bits((word.to_u64() >> LO) & Self::MASK)
    }

    /// Write `value` into the field, leaving every other bit of `word` — named,
    /// unnamed or reserved — exactly as it was.
    ///
    /// Infallible: the bound proves at compile time that no value of `T` can overrun
    /// the field. A type that *can* (a `u8` in a 7-bit slot) will not compile here.
    pub fn set<W: Word>(word: &mut W, value: T) {
        let () = Self::FITS;
        Self::write(word, value.to_bits());
    }

    /// Write `value` into the field, reporting an overflow rather than corrupting the
    /// neighbouring bits. The escape hatch for raw integers held in a narrower slot.
    pub fn checked_set<W: Word>(word: &mut W, value: T) -> Result<(), FieldOverflow> {
        let bits = value.to_bits();
        if bits > Self::MASK {
            return Err(FieldOverflow {
                value: bits,
                width: Self::WIDTH,
            });
        }
        Self::write(word, bits);
        Ok(())
    }

    fn write<W: Word>(word: &mut W, bits: u64) {
        let () = WordFits::<W, HI>::OK;
        let cleared = word.to_u64() & !(Self::MASK << LO);
        *word = W::from_u64(cleared | ((bits & Self::MASK) << LO));
    }
}

impl<T: Packed<Error = Infallible>, const HI: u32, const LO: u32> Field<T, HI, LO> {
    /// Decode the field, for types where every bit pattern is a valid value.
    pub fn read<W: Word>(word: W) -> T {
        match Self::get(word) {
            Ok(value) => value,
            Err(never) => match never {},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Flag = Field<bool, 4, 4>;
    type Nibble = Field<u8, 11, 8>;
    type Byte = Field<u8, 7, 0>;

    #[test]
    fn a_field_reads_only_its_own_bits() {
        assert_eq!(Nibble::read(0xabcd_u16), 0xb);
        assert_eq!(Byte::read(0xabcd_u16), 0xcd);
        assert!(Flag::read(0x0010_u16));
        assert!(!Flag::read(0xffef_u16));
    }

    #[test]
    fn a_write_disturbs_no_other_bit() {
        let mut word = 0xabcd_u16;
        Nibble::checked_set(&mut word, 0x3).unwrap();
        assert_eq!(word, 0xa3cd);

        // Including the bits no field names at all — the property that lets a panel
        // migrate one field at a time while unknown bits keep round-tripping.
        let mut word = 0b1010_1010_u8;
        Flag::set(&mut word, true);
        assert_eq!(word, 0b1011_1010);
        Flag::set(&mut word, false);
        assert_eq!(word, 0b1010_1010);
    }

    #[test]
    fn a_value_wider_than_its_field_is_refused_rather_than_truncated() {
        let mut word = 0u16;
        let err = Nibble::checked_set(&mut word, 0x1f).unwrap_err();
        assert_eq!(err.width, 4);
        assert_eq!(err.value, 0x1f);
        // The neighbouring bits are untouched by the failed write.
        assert_eq!(word, 0);
    }

    #[test]
    fn widths_and_masks_come_from_the_range_alone() {
        assert_eq!(Flag::WIDTH, 1);
        assert_eq!(Nibble::WIDTH, 4);
        assert_eq!(Nibble::MASK, 0xf);
        assert_eq!(Field::<u64, 63, 0>::MASK, u64::MAX);
    }

    #[test]
    fn bits_for_counts_what_a_value_needs() {
        assert_eq!(bits_for(0), 0);
        assert_eq!(bits_for(1), 1);
        assert_eq!(bits_for(12), 4);
        assert_eq!(bits_for(13), 4);
        assert_eq!(bits_for(127), 7);
        assert_eq!(bits_for(128), 8);
    }
}
