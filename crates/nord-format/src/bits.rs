//! Typed bit fields over a packed word.
//!
//! A [`Field`] names an inclusive bit range and owns both directions of the conversion,
//! so a field's position is written once. [`Straddle`] is the same thing for a value
//! split across two words.
//!
//! Bits are numbered from the least-significant bit of the word, so `HI`/`LO` read
//! straight off a big-endian hex dump once you know the word's byte span. Bits no field
//! names are left untouched.

use std::convert::Infallible;
use std::fmt::{self, Display, Formatter};
use std::marker::PhantomData;

/// A word that can host packed bit fields.
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

/// A value that can live inside a packed bit field.
///
/// Implementors own their encoding and validation and know nothing about which word or
/// offset holds them, so the same impl serves every field with that value.
pub trait Packed: Sized {
    /// Bits the widest value of this type occupies. Used to check statically that it
    /// fits its slot.
    const MAX_BITS: u32;

    /// Why a bit pattern is not a valid value. [`Infallible`] when every pattern is.
    type Error;

    /// Decode from the field's bits, already shifted down to bit 0 and masked.
    fn from_bits(bits: u64) -> Result<Self, Self::Error>;

    /// Encode to the field's bits, in the same shifted-down form.
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

/// A value too wide for the field it was written to. Writing it anyway would overrun
/// the neighbouring field.
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

/// One value packed into bits `LO..=HI`, inclusive.
///
/// Never instantiated — it names a position plus a conversion, used as
/// `MyField::get(word)` / `MyField::set(&mut word, v)`.
pub struct Field<T, const HI: u32, const LO: u32>(PhantomData<fn() -> T>);

/// Compile-time check that a field lies inside the word it is applied to.
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

    /// Compile-time check that every value of `T` fits. Forced by [`Self::set`].
    const FITS: () = assert!(
        T::MAX_BITS <= HI - LO + 1,
        "this type can hold values wider than the field; use `checked_set`",
    );

    /// Decode the field out of `word`.
    pub fn get<W: Word>(word: W) -> Result<T, T::Error> {
        let () = WordFits::<W, HI>::OK;
        T::from_bits((word.to_u64() >> LO) & Self::MASK)
    }

    /// Write `value`, leaving every other bit of `word` as it was.
    ///
    /// Only compiles when no value of `T` can overrun the field. A type that can — a
    /// `u8` in a 7-bit slot — must use [`Self::checked_set`].
    pub fn set<W: Word>(word: &mut W, value: T) {
        let () = Self::FITS;
        Self::write(word, value.to_bits());
    }

    /// Write `value`, reporting an overflow instead of corrupting the neighbouring
    /// bits. For raw integers in a narrower slot.
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

/// A position and a width, with no opinion about what the bits mean. Lets [`Straddle`]
/// compose a value out of two ranges.
pub trait BitRange {
    /// Width of the range in bits.
    const WIDTH: u32;

    /// The range's bits, shifted down to bit 0.
    fn raw<W: Word>(word: W) -> u64;

    /// Replace the range's bits, leaving the rest of the word alone.
    fn put_raw<W: Word>(word: &mut W, bits: u64);
}

impl<T, const HI: u32, const LO: u32> BitRange for Field<T, HI, LO> {
    const WIDTH: u32 = {
        assert!(HI >= LO, "bit field's HI must not be below its LO");
        HI - LO + 1
    };

    fn raw<W: Word>(word: W) -> u64 {
        let () = WordFits::<W, HI>::OK;
        (word.to_u64() >> LO) & mask(Self::WIDTH)
    }

    fn put_raw<W: Word>(word: &mut W, bits: u64) {
        let () = WordFits::<W, HI>::OK;
        let mask = mask(Self::WIDTH);
        *word = W::from_u64((word.to_u64() & !(mask << LO)) | ((bits & mask) << LO));
    }
}

/// A `width`-bit mask at bit 0.
const fn mask(width: u32) -> u64 {
    if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}

/// One value split across two words: `H` holds the high bits, `L` the low ones, so
/// `value == (high << L::WIDTH) | low`.
#[allow(clippy::type_complexity)]
pub struct Straddle<T, H, L>(PhantomData<fn() -> (T, H, L)>);

impl<T: Packed, H: BitRange, L: BitRange> Straddle<T, H, L> {
    /// Combined width of both halves.
    pub const WIDTH: u32 = H::WIDTH + L::WIDTH;

    /// As [`Field::FITS`], over the combined width.
    const FITS: () = assert!(
        T::MAX_BITS <= H::WIDTH + L::WIDTH,
        "this type can hold values wider than the field; use `checked_set`",
    );

    /// Decode the value out of the two words that hold it.
    pub fn get<A: Word, B: Word>(high: A, low: B) -> Result<T, T::Error> {
        T::from_bits((H::raw(high) << L::WIDTH) | L::raw(low))
    }

    /// Write across both words, disturbing nothing else in either.
    pub fn set<A: Word, B: Word>(high: &mut A, low: &mut B, value: T) {
        let () = Self::FITS;
        Self::write(high, low, value.to_bits());
    }

    /// As [`Field::checked_set`], over the combined width.
    pub fn checked_set<A: Word, B: Word>(
        high: &mut A,
        low: &mut B,
        value: T,
    ) -> Result<(), FieldOverflow> {
        let bits = value.to_bits();
        if bits > mask(Self::WIDTH) {
            return Err(FieldOverflow {
                value: bits,
                width: Self::WIDTH,
            });
        }
        Self::write(high, low, bits);
        Ok(())
    }

    fn write<A: Word, B: Word>(high: &mut A, low: &mut B, bits: u64) {
        H::put_raw(high, bits >> L::WIDTH);
        L::put_raw(low, bits);
    }
}

impl<T: Packed<Error = Infallible>, H: BitRange, L: BitRange> Straddle<T, H, L> {
    /// Decode, when every bit pattern is a valid value.
    pub fn read<A: Word, B: Word>(high: A, low: B) -> T {
        match Self::get(high, low) {
            Ok(value) => value,
            Err(never) => match never {},
        }
    }
}

impl<T: Packed<Error = Infallible>, const HI: u32, const LO: u32> Field<T, HI, LO> {
    /// Decode, when every bit pattern is a valid value.
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

    // A value split across a `u64`'s bottom three bits and a `u32`'s top four — the
    // shape of `equalizer_freq_gain`.
    type HighPart = Field<u64, 2, 0>;
    type LowPart = Field<u32, 31, 28>;
    type Spanning = Straddle<u8, HighPart, LowPart>;

    #[test]
    fn a_straddling_field_reassembles_both_halves() {
        assert_eq!(Spanning::WIDTH, 7);
        // high = 0b101, low = 0b1101  ->  0b101_1101
        assert_eq!(Spanning::read(0b101_u64, 0xd000_0000_u32), 0b1011101);
        assert_eq!(Spanning::read(0u64, 0u32), 0);
    }

    #[test]
    fn a_straddling_write_lands_in_both_words_and_nowhere_else() {
        let (mut high, mut low) = (0xffff_ffff_ffff_fff8_u64, 0x0fff_ffff_u32);
        Spanning::checked_set(&mut high, &mut low, 0b1011101).unwrap();
        assert_eq!(high, 0xffff_ffff_ffff_fffd);
        assert_eq!(low, 0xdfff_ffff);
        assert_eq!(Spanning::read(high, low), 0b1011101);
    }

    #[test]
    fn a_straddling_value_too_wide_is_refused() {
        let (mut high, mut low) = (0u64, 0u32);
        assert_eq!(
            Spanning::checked_set(&mut high, &mut low, 0x80)
                .unwrap_err()
                .width,
            7,
        );
        assert_eq!((high, low), (0, 0));
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
