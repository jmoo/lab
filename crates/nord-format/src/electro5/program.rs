use crate::bits::{Field, FieldOverflow};
use crate::common;
use crate::common::{bank, PartMix};
use crate::crc::{CrcReader, CrcWriter};
use crate::electro5::{Instrument, OctaveShift, SplitPoint, Transpose};
use crate::error::ParseError;
use crate::types::RangedU16Pair;
use binrw::{binrw, BinRead, BinReaderExt, BinWriterExt};

use std::fmt::Debug;
use std::io;

pub const FORMAT: &str = "ne5p";
/// Schema versions this build's field offsets have been validated against. Every one of
/// the 624 corpus programs reports 4. See [`crate::error::ParseError::UnsupportedVersion`].
pub const KNOWN_VERSIONS: &[u32] = &[4];
/// Total file length: 44-byte CBIN header + 121-byte body.
pub const FILE_LEN: usize = 165;
pub const BANK_COUNT: u16 = 8;
pub const SLOT_COUNT: u16 = 50;

pub type Location = RangedU16Pair<BANK_COUNT, SLOT_COUNT>;
pub type Header = common::Header<Location>;
pub type Bank = bank::Bank<Program, Location>;

// 0x2e-0x34 — the centre panel. RFC-0001's option **B+** prototype: the three backing
// words are private and every logical value is a `Field<T, HI, LO>` with a getter *and*
// a setter, so an assignment lands in the bytes instead of being discarded.
//
// Before this, the panel decoded into ~15 `pub` fields carrying `#[bw(ignore)]`, and
// which of them survived a write depended on whether their word had a `bw(calc)`:
// `settings` and `settings2` were recomputed from their decoded fields, `settings3` was
// stored verbatim, so `panel.gain = 3` compiled, changed nothing, and reported success.
// Identical syntax, opposite semantics, nothing in the type system marking the
// difference. Now there are no public decoded fields to assign to at all.
//
// Bits are numbered from the LSB of each word, which for these big-endian words means
// `HI`/`LO` map directly onto the old `0b…` masks. Unnamed bits — `settings3` 7..0, and
// `unknown_boolean1` aside, whatever else is not listed — are simply not fields; they
// stay in the word untouched and round-trip verbatim.

/// 0x2e..0x2f — parts, octave shifts, sustain.
type LeftPart = Field<Instrument, 15, 13>;
type RightPart = Field<Instrument, 12, 10>;
type LeftOctaveShift = Field<OctaveShift, 9, 6>;
type RightOctaveShift = Field<OctaveShift, 5, 2>;
type LeftSustain = Field<bool, 1, 1>;
type RightSustain = Field<bool, 0, 0>;

/// 0x30 — pedals, split, transpose enable.
type LeftControl = Field<bool, 7, 7>;
type RightControl = Field<bool, 6, 6>;
type UnknownBoolean1 = Field<bool, 5, 5>;
type Split = Field<bool, 4, 4>;
type SplitPointField = Field<SplitPoint, 3, 1>;
type TransposeEnabled = Field<bool, 0, 0>;

/// 0x31..0x34 — transpose, part mix, gain, organ selection.
type TransposeField = Field<Transpose, 31, 28>;
type PartMixField = Field<PartMix, 27, 21>;
type Gain = Field<u8, 20, 14>;
type OrganType = Field<u8, 13, 11>;
type LowerEnabled = Field<bool, 10, 10>;
type UpperEnabled = Field<bool, 9, 9>;
type DrawbarLive = Field<bool, 8, 8>;

#[binrw]
pub struct CenterPanel {
    // 0x2e..0x2f                 0x2e     0x2f
    #[brw(big)]
    settings: u16,

    // 0x30                    0x30
    #[brw(big)]
    settings2: u8,

    // 0x31..34                     0x31      0x32      0x33     0x34
    #[brw(big)]
    settings3: u32,
}

impl CenterPanel {
    pub fn left_part(&self) -> Result<Instrument, ParseError> {
        LeftPart::get(self.settings)
    }

    pub fn set_left_part(&mut self, value: Instrument) {
        LeftPart::set(&mut self.settings, value)
    }

    pub fn right_part(&self) -> Result<Instrument, ParseError> {
        RightPart::get(self.settings)
    }

    pub fn set_right_part(&mut self, value: Instrument) {
        RightPart::set(&mut self.settings, value)
    }

    pub fn left_octave_shift(&self) -> Result<OctaveShift, ParseError> {
        LeftOctaveShift::get(self.settings)
    }

    pub fn set_left_octave_shift(&mut self, value: OctaveShift) {
        LeftOctaveShift::set(&mut self.settings, value)
    }

    pub fn right_octave_shift(&self) -> Result<OctaveShift, ParseError> {
        RightOctaveShift::get(self.settings)
    }

    pub fn set_right_octave_shift(&mut self, value: OctaveShift) {
        RightOctaveShift::set(&mut self.settings, value)
    }

    pub fn left_sustain(&self) -> bool {
        LeftSustain::read(self.settings)
    }

    pub fn set_left_sustain(&mut self, value: bool) {
        LeftSustain::set(&mut self.settings, value)
    }

    pub fn right_sustain(&self) -> bool {
        RightSustain::read(self.settings)
    }

    pub fn set_right_sustain(&mut self, value: bool) {
        RightSustain::set(&mut self.settings, value)
    }

    pub fn left_control(&self) -> bool {
        LeftControl::read(self.settings2)
    }

    pub fn set_left_control(&mut self, value: bool) {
        LeftControl::set(&mut self.settings2, value)
    }

    pub fn right_control(&self) -> bool {
        RightControl::read(self.settings2)
    }

    pub fn set_right_control(&mut self, value: bool) {
        RightControl::set(&mut self.settings2, value)
    }

    /// Always zero in every corpus specimen. Named so it is visible, not so it is used.
    pub fn unknown_boolean1(&self) -> bool {
        UnknownBoolean1::read(self.settings2)
    }

    pub fn split(&self) -> bool {
        Split::read(self.settings2)
    }

    pub fn set_split(&mut self, value: bool) {
        Split::set(&mut self.settings2, value)
    }

    pub fn split_point(&self) -> Result<SplitPoint, ParseError> {
        SplitPointField::get(self.settings2)
    }

    pub fn set_split_point(&mut self, value: SplitPoint) {
        SplitPointField::set(&mut self.settings2, value)
    }

    /// NOTE: the Electro 5 sometimes leaves this true even when the transpose is 0. It
    /// shows no transpose light when that happens.
    pub fn transpose_enabled(&self) -> bool {
        TransposeEnabled::read(self.settings2)
    }

    pub fn set_transpose_enabled(&mut self, value: bool) {
        TransposeEnabled::set(&mut self.settings2, value)
    }

    /// Half-step transposition, `-6..=6`, stored biased by 6.
    pub fn transpose(&self) -> Result<Transpose, ParseError> {
        TransposeField::get(self.settings3)
    }

    pub fn set_transpose(&mut self, value: Transpose) {
        TransposeField::set(&mut self.settings3, value)
    }

    pub fn part_mix(&self) -> Result<PartMix, ParseError> {
        PartMixField::get(self.settings3)
    }

    pub fn set_part_mix(&mut self, value: PartMix) {
        PartMixField::set(&mut self.settings3, value)
    }

    /// 0..=127, shown on the panel as 0..10.
    pub fn gain(&self) -> u8 {
        Gain::read(self.settings3)
    }

    /// Fallible because the slot is seven bits and a `u8` is eight: 128 or more would
    /// overrun into [`Self::part_mix`]. This is the write-side width validation
    /// RFC-0001 lists as a new requirement — nothing checked it before because nothing
    /// wrote.
    pub fn set_gain(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Gain::checked_set(&mut self.settings3, value)
    }

    /// `0` b3, `1` b3+bass, `2` pipe, `3` vox, `4` farfisa.
    pub fn organ_type(&self) -> u8 {
        OrganType::read(self.settings3)
    }

    pub fn set_organ_type(&mut self, value: u8) -> Result<(), FieldOverflow> {
        OrganType::checked_set(&mut self.settings3, value)
    }

    pub fn lower_enabled(&self) -> bool {
        LowerEnabled::read(self.settings3)
    }

    pub fn set_lower_enabled(&mut self, value: bool) {
        LowerEnabled::set(&mut self.settings3, value)
    }

    pub fn upper_enabled(&self) -> bool {
        UpperEnabled::read(self.settings3)
    }

    pub fn set_upper_enabled(&mut self, value: bool) {
        UpperEnabled::set(&mut self.settings3, value)
    }

    pub fn drawbar_live(&self) -> bool {
        DrawbarLive::read(self.settings3)
    }

    pub fn set_drawbar_live(&mut self, value: bool) {
        DrawbarLive::set(&mut self.settings3, value)
    }

    /// Decode every fallible field, reporting the first that does not hold a valid
    /// value.
    ///
    /// Decoding used to happen inside `binrw`'s `try_calc`, so a bad value failed the
    /// *read*. Now that values are decoded on demand, that check has to be made
    /// explicitly — [`Program::read_from`] calls this, which keeps the old contract: a
    /// `Program` that was read successfully decodes cleanly, and the only way to change
    /// a field afterwards is through a typed setter.
    pub fn validate(&self) -> Result<(), ParseError> {
        self.left_part()?;
        self.right_part()?;
        self.left_octave_shift()?;
        self.right_octave_shift()?;
        self.split_point()?;
        self.transpose()?;
        self.part_mix()?;
        Ok(())
    }
}

/// Coherent defaults, built through the setters rather than left as a zeroed word.
///
/// A zeroed `settings` decodes to an octave shift of `-7`, which is out of range — the
/// old `Default` only avoided that because the write path recomputed the word from the
/// decoded fields and the read path never saw the zeros. With the word authoritative,
/// the default has to be a real default.
///
/// One behavioural change falls out of this: a program from [`Program::new`] now has
/// `transpose = 0`, where before `settings3` was left at zero and decoded as `-6`.
impl Default for CenterPanel {
    fn default() -> Self {
        let mut panel = CenterPanel {
            settings: 0,
            settings2: 0,
            settings3: 0,
        };
        panel.set_left_part(Instrument::default());
        panel.set_right_part(Instrument::default());
        panel.set_left_octave_shift(OctaveShift::default());
        panel.set_right_octave_shift(OctaveShift::default());
        panel.set_split_point(SplitPoint::default());
        panel.set_transpose(Transpose::default());
        panel.set_part_mix(PartMix::default());
        panel
    }
}

/// Renders a decoded value, or its error, in place of the raw word.
struct Decoded<T, E>(Result<T, E>);

impl<T: Debug, E: std::fmt::Display> Debug for Decoded<T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Ok(value) => Debug::fmt(value, f),
            Err(e) => write!(f, "<invalid: {e}>"),
        }
    }
}

/// Hand-written so a panel dump still shows decoded values rather than three integers.
/// A field that does not decode prints as `<invalid: …>` instead of aborting the dump.
impl Debug for CenterPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CenterPanel")
            .field("left_part", &Decoded(self.left_part()))
            .field("right_part", &Decoded(self.right_part()))
            .field("left_octave_shift", &Decoded(self.left_octave_shift()))
            .field("right_octave_shift", &Decoded(self.right_octave_shift()))
            .field("left_sustain", &self.left_sustain())
            .field("right_sustain", &self.right_sustain())
            .field("left_control", &self.left_control())
            .field("right_control", &self.right_control())
            .field("split", &self.split())
            .field("split_point", &Decoded(self.split_point()))
            .field("transpose", &Decoded(self.transpose()))
            .field("transpose_enabled", &self.transpose_enabled())
            .field("part_mix", &Decoded(self.part_mix()))
            .field("gain", &self.gain())
            .field("organ_type", &self.organ_type())
            .field("lower_enabled", &self.lower_enabled())
            .field("upper_enabled", &self.upper_enabled())
            .field("drawbar_live", &self.drawbar_live())
            .finish()
    }
}

// 0x3a..0x41
#[binrw]
#[derive(Debug, Default)]
pub struct PianoPanel {
    // 0x3a..0x41               0x3a      0x3b     0x3c     0x3d     0x3e     0x3f     0x40    0x41
    #[brw(big)]
    settings: u64,

    // 5 == 0, 6 == 1, 1 == 2, 2 == 3, 3 == 4, 4 == 5
    #[br(calc = ((settings & 0b11100000_00000000_00000000_00000000_00000000_00000000_00000000_00000000) >> ((8 * 7) + 5)) as u8)]
    #[bw(ignore)]
    pub category: u8,

    /// Zero-based model slot *within* [`category`](Self::category) — the panel's
    /// Model dial. A slot coordinate, not an identity; see [`id`](Self::id).
    #[br(calc = ((settings & 0b00000111_11000000_00000000_00000000_00000000_00000000_00000000_00000000) >> ((8 * 6) + 6)) as u8)]
    #[bw(ignore)]
    pub piano_model: u8,

    #[br(calc = ((settings & 0b00000000_00000001_10000000_00000000_00000000_00000000_00000000_00000000) >> ((8 * 5) + 7)) as u8)]
    #[bw(ignore)]
    pub clav_model: u8,

    #[br(calc = ((settings & 0b00000000_00000000_01100000_00000000_00000000_00000000_00000000_00000000) >> ((8 * 5) + 5)) as u8)]
    #[bw(ignore)]
    pub acoustics: u8,

    #[br(calc = ((settings & 0b00000000_00000000_00011000_00000000_00000000_00000000_00000000_00000000) >> ((8 * 5) + 3)) as u8)]
    #[bw(ignore)]
    pub touch: u8,

    #[br(calc = ((settings & 0b00000000_00000000_00000100_00000000_00000000_00000000_00000000_00000000) >> ((8 * 5) + 2)) != 0)]
    #[bw(ignore)]
    pub mono: bool,

    /// The piano (`.npno`) this program depends on: a stable 32-bit id in bits
    /// 41..=10, independent of where the piano sits in the instrument's library.
    /// `0` means "no piano referenced". This — not
    /// [`category`](Self::category)/[`piano_model`](Self::piano_model), which are
    /// slot coordinates — is what resolves the song → program → piano chain, and
    /// what Nord Sound Manager checks to decide whether a Restore is missing a
    /// dependency.
    #[br(calc = ((settings & 0b00000000_00000000_00000011_11111111_11111111_11111111_11111100_00000000) >> ((8 * 1) + 2)) as u32)]
    #[bw(ignore)]
    pub id: u32,
}

// 0x46..0x4d
#[binrw]
#[derive(Debug, Default)]
pub struct SamplePanel {
    // 0x46..0x4d               0x46      0x47     0x48     0x49     0x4a     0x4b     0x4c    0x4d
    #[brw(big)]
    settings: u64,

    #[br(calc = ((settings & 0b11111110_00000000_00000000_00000000_00000000_00000000_00000000_00000000) >> ((8 * 7) + 1)) as u8)]
    #[bw(ignore)]
    pub attack: u8,

    #[br(calc = ((settings & 0b00000001_11111100_00000000_00000000_00000000_00000000_00000000_00000000) >> ((8 * 6) + 2)) as u8)]
    #[bw(ignore)]
    pub decay_release: u8,

    /// Zero-based slot of the sample in the instrument's Samp Lib, i.e. the
    /// number shown on the panel minus one. This is a *position*, not an
    /// identity: adding or deleting samples renumbers it, and the corpus has
    /// ids that appear under several numbers (and numbers reused by several
    /// ids). Use [`id`](Self::id) to resolve the dependency.
    #[br(calc = ((settings & 0b00000000_00000011_11111100_00000000_00000000_00000000_00000000_00000000) >> ((8 * 5) + 2)) as u8)]
    #[bw(ignore)]
    pub number: u8,

    /// The sample (`.nsmp`) this program depends on: a stable 32-bit id in bits
    /// 41..=10, laid out exactly as [`PianoPanel::id`]. `0` means "no sample
    /// referenced".
    #[br(calc = ((settings & 0b00000000_00000000_00000011_11111111_11111111_11111111_11111100_00000000) >> ((8 * 1) + 2)) as u32)]
    #[bw(ignore)]
    pub id: u32,

    #[br(calc = ((settings & 0b00000000_00000000_00000000_00000000_00000000_00000000_00000011_00000000) >> ((8 * 1) + 0)) as u8)]
    #[bw(ignore)]
    pub dynamics: u8,

    #[br(calc = ((settings & 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_10000000) >> ((8 * 0) + 7)) != 0)]
    #[bw(ignore)]
    pub filter: bool,
}

// 0x4e..0x92 — the organ panel. The Electro 5 stores the full drawbar +
// vib/perc state for *every* organ model (B3, Vox, Farfisa, Pipe) and both
// presets, so switching model/preset on the instrument is lossless too.
//
//   * Drawbars = 9 nibbles, physical position 0..=8, packed high-nibble first,
//     at these panel offsets per model + preset (B3-bass shares the B3 slots):
//         B3   p1 0x55  p2 0x5c      Vox  p1 0x67  p2 0x6d
//         Farf p1 0x77  p2 0x7d      Pipe p1 0x87  p2 0x8d
//     Every model stores the *physical* bar position on disk; the per-model
//     "real" value (Farf's >=5 on/off, Vox's ignored 8th bar, B3-bass's remapped
//     bass bars) is a display transform layered on top — NOT decoded here yet.
//   * Preset selection = bit 0x40 of one byte per model group
//         B3 0x53, Vox 0x65, Farf 0x75, Pipe 0x85   (0 = preset 1, 1 = preset 2)
//
// STILL RAW (byte map retained below): the vib/chorus on-off + type and perc
// on/third/speed toggles. They round-trip byte-exact through `raw`; decoding
// them semantically is the next organ increment.

/// Length of the organ panel block, 0x4e..=0x92 (69 bytes).
const ORGAN_LEN: usize = 0x92 - 0x4d;

/// Panel-relative index of the byte at absolute Electro 5 file offset `abs`
/// (the organ panel begins at 0x4e).
const fn org(abs: usize) -> usize {
    abs - 0x4e
}

/// Nine drawbar positions (physical, 0..=8), nibble-packed high-nibble first,
/// starting at panel-relative byte `at`. This is the on-disk form shared by all
/// organ models; per-model display transforms are applied elsewhere.
fn read_drawbars(raw: &[u8], at: usize) -> [u8; 9] {
    let mut bars = [0u8; 9];
    for (n, bar) in bars.iter_mut().enumerate() {
        let byte = raw[at + n / 2];
        *bar = if n % 2 == 0 { byte >> 4 } else { byte & 0x0f };
    }
    bars
}

/// The Electro 5's four organ models. (B3-bass shares the B3 storage slots, so
/// it isn't a separate model here.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrganModel {
    B3,
    Vox,
    Farfisa,
    Pipe,
}

#[binrw]
#[derive(Debug)]
pub struct OrganPanel {
    /// The whole 0x4e..=0x92 block, kept verbatim so the panel always
    /// round-trips byte-exact. Decoded values are exposed via the methods below.
    raw: [u8; ORGAN_LEN],
    // // 0x4e..0x50
    // pad: B24,
    //
    // // 0x51 0b11100000
    // pub b3_vib_type: B3,
    //
    // // 0x51 0b00010000
    // pub b3_perc_third: bool,
    //
    // // 0x51 0b00001100
    // pub b3_perc_speed: B3,
    //
    // // 0x52
    // pad2: u8,
    //
    // // 0x53 0b01000000
    // pub b3_bass_preset: bool,
    //
    // // 0x54
    // pub unknown_byte: u8,
    //
    // // 0x55 0b11111111_11111111_11111111_11111111_11110000
    // pub preset1_b3_drawbars: Drawbars,

    // Drawbars: 9 with 4 bits each representing a value of 0..8
    // 0x4e..0x50      - pad
    // 0x51 0b11100000 - preset 1/2 b3/b3-bass vib selection (010: 0, 101: 3)
    // 0x51 0b00010000 - preset 1/2 b3/b3-bass perc third (0,1)
    // 0x51 0b00001100 - preset 1/2 b3/b3-bass perc speed (10: 1, 01: 2, 11: 3)
    // 0x52 0b00000000 - pad
    // 0x53 0b01000000 - b3/b3-bass preset selection
    // 0x54 0b00000000 - ?
    // 0x55 0b11111111_11111111_11111111_11111111_11110000 - preset1 drawbars (b3 normal, b3-bass inverted for first two and then normal for the rest except their value is ignored)
    // 0x59 0b00001000 - preset 1 b3/b3-bass vib on/off (0,1)
    // 0x59 0b00000100 - preset 1 b3/b3-bass perc on/off (0,1)
    // 0x59 0b00000010 - ?
    // 0x59 0b00000001 - ?
    // 0x5a 0b00000000 - ?
    // 0x5b 0b00000000 - pad
    // 0x5c 0b11111111_11111111_11111111_11111111_11110000 - preset2 drawbars (b3 normal, b3-bass normal)
    // 0x60 0b00001000 - preset 2 b3/b3-bass vib on/off (0,1)
    // 0x60 0b00000100 - preset 2 b3/b3-bass perc on/off (0,1)
    // 0x60 0b00000010 - unknown boolean (true on all programs i have created, false on a bunch of random presets)
    // 0x61 0b00100000 - unknown boolean (true on all programs i have created, false on a bunch of random presets)
    // 0x62 0b00000000 - pad
    // 0x63 0b11100000 - preset 1/2 vox vib selection (000: 4, 010: 2, 001: 0)
    // 0x64 0b00000000 - pad
    // 0x65 0b01000000 - vox preset selection
    // 0x66 0b00000000 - pad
    // 0x67 0b11111111_11111111_11111111_11111111_11110000 - preset1 drawbars (vox normal but 8th drawbar value is ignored)
    // 0x6b 0b00001000 - preset 1 vox vib on/off
    // 0x6c 0b00000000 - pad
    // 0x6d 0b11111111_11111111_11111111_11111111_11110000 - preset1 drawbars (vox normal but 8th drawbar value is ignored)
    // 0x71 0b00001000 - preset 2 vox vib on/off
    // 0x72 0b00000000 - pad
    // 0x73 0b11100000 - preset 1/2 farfisa vib selection (000: 4, 011: 3, 010: 1, 001: 0)
    // 0x74 0b00000000 - pad
    // 0x75 0b01000000 - farf preset selection
    // 0x76 0b00000000 - pad
    // 0x77 0b11111111_11111111_11111111_11111111_11110000 - preset1 drawbars (farfisa normal values but >= 5 is interpreted as 1 and anything else is interpreted as 0)
    // 0x7b 0b00001000 - preset 1 farfisa vib on/off
    // 0x7c 0b00000000 - pad
    // 0x7d 0b11111111_11111111_11111111_11111111_11110000 - preset2 drawbars (farfisa normal values but >= 5 is interpreted as 1 and anything else is interpreted as 0)
    // 0x81 0b00001000 - preset 2 farfisa vib on/off
    // 0x82 0b00000000 - pad
    // 0x83 0b00000000 - pad
    // 0x84 - pad
    // 0x85 0b01000000 - pipe preset selection
    // 0x86 - pad
    // 0x87 0b11111111_11111111_11111111_11111111_11110000 - preset1 drawbars (pipe normal)
    // 0x8b 0b00001000 - unknown boolean (always true except for included preset 'Sunday')
    // 0x8c 0b00000000 - pad
    // 0x8d 0b11111111_11111111_11111111_11111111_11110000 - preset2 drawbars (pipe, normal)
}

/// A vibrato (`V`) or chorus (`C`) organ modulation at one of three depths.
/// Which subset is available depends on the model (see [`OrganPanel::vib_type`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VibChorus {
    V1,
    C1,
    V2,
    C2,
    V3,
    C3,
}

/// B3 percussion decay speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PercSpeed {
    Off,
    Soft,
    Fast,
    Both,
}

impl OrganPanel {
    /// Panel-relative drawbar-block offset for a model + preset (1 or 2).
    fn drawbar_offset(model: OrganModel, preset: u8) -> usize {
        let (p1, p2) = match model {
            OrganModel::B3 => (0x55, 0x5c),
            OrganModel::Vox => (0x67, 0x6d),
            OrganModel::Farfisa => (0x77, 0x7d),
            OrganModel::Pipe => (0x87, 0x8d),
        };
        org(if preset == 2 { p2 } else { p1 })
    }

    /// Panel-relative index of a model's preset-selection byte (bit 0x40).
    fn preset_byte(model: OrganModel) -> usize {
        org(match model {
            OrganModel::B3 => 0x53,
            OrganModel::Vox => 0x65,
            OrganModel::Farfisa => 0x75,
            OrganModel::Pipe => 0x85,
        })
    }

    /// The selected preset (1 or 2) for `model`.
    pub fn preset(&self, model: OrganModel) -> u8 {
        if self.raw[Self::preset_byte(model)] & 0x40 != 0 {
            2
        } else {
            1
        }
    }

    /// The nine drawbar positions (physical, 0..=8) stored for `model`'s
    /// `preset`. This is the on-disk value; per-model display transforms
    /// (Farfisa on/off, Vox's ignored 8th bar, B3-bass bass-bar remap) are not
    /// applied.
    pub fn drawbars(&self, model: OrganModel, preset: u8) -> [u8; 9] {
        read_drawbars(&self.raw, Self::drawbar_offset(model, preset))
    }

    /// The two bass drawbars of **B3-with-bass, preset 1** — the bass manual.
    ///
    /// These are *not* in the nine-nibble block. In b3+bass mode preset 1 is the bass
    /// manual (only bars 1–2 are live) and preset 2 is the ordinary B3; the bass
    /// registration lives in a 12-bit field spanning the low nibble of `0x59` and all
    /// of `0x5a`:
    ///
    /// ```text
    /// field = ((raw[0x59] & 0x0F) << 8) | raw[0x5a]
    /// bar1  = (field >> 6) & 0xF      bar2 = (field >> 2) & 0xF
    /// ```
    ///
    /// Values are identity (`8` stores `8`); the low 2 bits of the field are unused.
    /// The `& 0xF` masks matter: `0x59` also carries B3 vibrato-on (`0x08`) and
    /// percussion-on (`0x04`), which land at field bits 11 and 10, and its *high*
    /// nibble is bar 9 of the main block.
    ///
    /// ⚠️ Do **not** read bars 1–2 from [`Self::drawbars`] in this mode — those two
    /// nibbles hold stale leftovers, not zero and not the bass values.
    ///
    /// Derived by diffing corpus specimens and confirmed against hardware captures
    /// `1100_400000000` / `1100_040000000`.
    pub fn b3_bass_drawbars(&self) -> [u8; 2] {
        let field = ((self.raw[org(0x59)] as u16 & 0x0F) << 8) | self.raw[org(0x5a)] as u16;
        [((field >> 6) & 0xF) as u8, ((field >> 2) & 0xF) as u8]
    }

    /// Farfisa drawbars as the instrument actually treats them: **on/off tabs**, not
    /// continuous positions.
    ///
    /// A stored nibble of **≥5 reads as on**, anything lower as off. Use this rather
    /// than [`Self::drawbars`] for Farfisa — the raw 0..=8 value is stored faithfully
    /// but has no meaning beyond which side of the threshold it falls.
    pub fn farfisa_tabs(&self, preset: u8) -> [bool; 9] {
        let bars = self.drawbars(OrganModel::Farfisa, preset);
        let mut tabs = [false; 9];
        for (tab, bar) in tabs.iter_mut().zip(bars) {
            *tab = bar >= 5;
        }
        tabs
    }

    /// Panel-relative index of the per-preset vib/perc byte for `model`, or
    /// `None` for Pipe (no vib/perc). For B3 this byte also holds percussion.
    fn effect_byte(model: OrganModel, preset: u8) -> Option<usize> {
        let (p1, p2) = match model {
            OrganModel::B3 => (0x59, 0x60),
            OrganModel::Vox => (0x6b, 0x71),
            OrganModel::Farfisa => (0x7b, 0x81),
            OrganModel::Pipe => return None,
        };
        Some(org(if preset == 2 { p2 } else { p1 }))
    }

    /// Whether vibrato/chorus is on for `model`'s `preset`.
    pub fn vib_on(&self, model: OrganModel, preset: u8) -> bool {
        match Self::effect_byte(model, preset) {
            Some(i) => self.raw[i] & 0x08 != 0,
            None => false,
        }
    }

    /// The vibrato/chorus mode selected for `model` (shared across presets), or
    /// `None` for Pipe. Each model exposes a different subset of the six modes,
    /// so the stored 3-bit value indexes into a per-model table.
    pub fn vib_type(&self, model: OrganModel) -> Option<VibChorus> {
        use VibChorus::*;
        let (byte, table): (usize, &[VibChorus]) = match model {
            OrganModel::B3 => (org(0x51), &[V1, C1, V2, C2, V3, C3]),
            OrganModel::Vox => (org(0x63), &[V1, V2, V3]),
            OrganModel::Farfisa => (org(0x73), &[V1, V2, C2, C3]),
            OrganModel::Pipe => return None,
        };
        table.get((self.raw[byte] >> 5) as usize).copied()
    }

    /// Whether B3 percussion is on for `preset` (B3 only).
    pub fn b3_perc_on(&self, preset: u8) -> bool {
        self.raw[org(if preset == 2 { 0x60 } else { 0x59 })] & 0x04 != 0
    }

    /// Whether B3 percussion uses the third harmonic (shared across presets).
    pub fn b3_perc_third(&self) -> bool {
        self.raw[org(0x51)] & 0x10 != 0
    }

    /// B3 percussion decay speed (shared across presets). Note the on-disk
    /// encoding is not monotonic — the two speed bits store 2/1/3 for
    /// soft/fast/both.
    pub fn b3_perc_speed(&self) -> PercSpeed {
        match (self.raw[org(0x51)] >> 2) & 0x03 {
            0 => PercSpeed::Off,
            2 => PercSpeed::Soft,
            1 => PercSpeed::Fast,
            _ => PercSpeed::Both,
        }
    }
}

// 0x93..0x9F
#[binrw]
#[derive(Debug, Default)]
pub struct EffectsPanel {
    // 0x93..0x9a               0x93      0x94     0x95     0x96     0x97     0x98     0x99    0x9a
    #[brw(big)]
    settings: u64,

    // fx1 (0: off, 1: lower, 2: upper)
    #[br(calc = ((settings & 0b11000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000) >> ((8 * 7) + 6)) as u8)]
    #[bw(ignore)]
    pub fx1: u8,

    // fx1 type (1: pan1, pan2, pan1&2, 2: wah, rm, trem1, trem2, trem1&2)
    #[br(calc = ((settings & 0b00111100_00000000_00000000_00000000_00000000_00000000_00000000_00000000) >> ((8 * 7) + 2)) as u8)]
    #[bw(ignore)]
    pub fx1_type: u8,

    // fx1 rate 0..127 (0..10)
    #[br(calc = ((settings & 0b00000011_11111000_00000000_00000000_00000000_00000000_00000000_00000000) >> ((8 * 6) + 3)) as u8)]
    #[bw(ignore)]
    pub fx1_rate: u8,

    // fx2 (0: off, 1: lower, 2: upper
    #[br(calc = ((settings & 0b00000000_00000110_00000000_00000000_00000000_00000000_00000000_00000000) >> ((8 * 6) + 1)) as u8)]
    #[bw(ignore)]
    pub fx2: u8,

    // fx2 type (flang, choir1, choir2, vibe, phas1, phas2)
    #[br(calc = ((settings & 0b00000000_00000001_11100000_00000000_00000000_00000000_00000000_00000000) >> ((8 * 5) + 5)) as u8)]
    #[bw(ignore)]
    pub fx2_type: u8,

    // fx2 rate 0..127 (0..10)
    #[br(calc = ((settings & 0b00000000_00000000_00011111_11000000_00000000_00000000_00000000_00000000) >> ((8 * 4) + 6)) as u8)]
    #[bw(ignore)]
    pub fx2_rate: u8,

    // fx4 (0: off, 1: lower, 2: upper)
    #[br(calc = ((settings & 0b00000000_00000000_00000000_00110000_00000000_00000000_00000000_00000000) >> ((8 * 4) + 4)) as u8)]
    #[bw(ignore)]
    pub fx4: u8,

    #[br(calc = ((settings & 0b00000000_00000000_00000000_00001100_00000000_00000000_00000000_00000000) >> ((8 * 4) + 2)) as u8)]
    #[bw(ignore)]
    pub fx4_feedback: u8,

    // fx4 rate 0..127 (750ms..20ms)
    #[br(calc = ((settings & 0b00000000_00000000_00000000_00000011_11111000_00000000_00000000_00000000) >> ((8 * 3) + 3)) as u8)]
    #[bw(ignore)]
    pub fx4_tempo: u8,

    // fx4 wet/dry 0..127 (0..10)
    #[br(calc = ((settings & 0b00000000_00000000_00000000_00000000_00000111_11110000_00000000_00000000) >> ((8 * 2) + 4)) as u8)]
    #[bw(ignore)]
    pub fx4_moisture: u8,

    #[br(calc = ((settings & 0b00000000_00000000_00000000_00000000_00000000_00001000_00000000_00000000) >> ((8 * 2) + 3)) != 0)]
    #[bw(ignore)]
    pub fx4_ping_pong: bool,

    /// EQ engaged. **Which part it applies to is not in this word** — see
    /// [`Extra::equalizer_part`].
    ///
    /// This was previously decoded as a two-bit `equalizer_part_select`, which could
    /// only ever answer 0 or 2: diffing the four named `equalizer/{0,1,2,3}_…`
    /// specimens shows they are byte-identical across `0x93..0x9a` apart from this
    /// single bit, and the lower/upper/both choice lives at `0xa1`.
    #[br(calc = ((settings & 0b00000000_00000000_00000000_00000000_00000000_00000100_00000000_00000000) >> ((8 * 2) + 2)) != 0)]
    #[bw(ignore)]
    pub equalizer_on: bool,

    #[br(calc = ((settings & 0b00000000_00000000_00000000_00000000_00000000_00000001_11111100_00000000) >> ((8 * 1) + 2)) as u8)]
    #[bw(ignore)]
    pub equalizer_freq: u8,

    #[br(calc = ((settings & 0b00000000_00000000_00000000_00000000_00000000_00000000_00000011_11111000) >> ((8 * 0) + 3)) as u8)]
    #[bw(ignore)]
    pub equalizer_treble: u8,

    // 0x9b..0x9e
    #[brw(big)]
    settings2: u32,

    //                           0x9a                                     0x9b      0x9c     0x9d      0x9e
    #[br(calc = (((settings & 0b00000111) << 4) as u8) + (((settings2 & 0b11110000_00000000_00000000_00000000) >> ((8 * 3) + 4)) as u8))]
    #[bw(ignore)]
    pub equalizer_freq_gain: u8,

    //                           0x9b      0x9c      0x9d     0x9e
    #[br(calc = ((settings2 & 0b00001111_11100000_00000000_00000000) >> ((8 * 2) + 5)) as u8)]
    #[bw(ignore)]
    pub equalizer_bass: u8,

    // fx3 (0: off, 1: lower, 2: upper)
    #[br(calc = ((settings2 & 0b00000000_00011000_00000000_00000000) >> ((8 * 2) + 3)) as u8)]
    #[bw(ignore)]
    pub fx3: u8,

    // fx3 type (none, twin, rotary, comp, small, jc)
    #[br(calc = ((settings2 & 0b00000000_00000111_00000000_00000000) >> ((8 * 2) + 0)) as u8)]
    #[bw(ignore)]
    pub fx3_type: u8,

    // fx3 rate 0..127 (0..10)
    #[br(calc = ((settings2 & 0b00000000_00000000_11111110_00000000) >> ((8 * 1) + 1)) as u8)]
    #[bw(ignore)]
    pub fx3_compression: u8,

    #[br(calc = ((settings2 & 0b00000000_00000000_00000001_00000000) >> ((8 * 1) + 0)) != 0)]
    #[bw(ignore)]
    pub fx5: bool,

    #[br(calc = ((settings2 & 0b00000000_00000000_00000000_11100000) >> ((8 * 0) + 5)) as u8)]
    #[bw(ignore)]
    pub fx5_type: u8,

    // 0x9f
    #[brw(big)]
    settings3: u8,

    // 0x9b..0x9f                  0x9b      0x9c      0x9d     0x9e                                   0x9f
    #[br(calc = ((((settings2 & 0b00000000_00000000_00000000_00011111)) << 2) as u8) + ((settings3 & 0b11000000) >> 6))]
    #[bw(ignore)]
    pub fx5_moisture: u8,

    // 0 = off, 1 = on
    #[br(calc = ((settings3 & 0b00100000) >> ((8 * 0) + 5)) as u8)]
    #[bw(ignore)]
    pub rotary_stop: u8,

    // 0 = slow, 1 = fast
    #[br(calc = ((settings3 & 0b00010000) >> ((8 * 0) + 4)) as u8)]
    #[bw(ignore)]
    pub rotary_speed: u8,
}

// 0xa1..0xa4
#[binrw]
#[derive(Debug, Default)]
pub struct Extra {
    #[brw(big)]
    settings: u32,

    // fx1 control pedal (0: off, 1: on)
    #[br(calc = ((settings & 0b00010000_00000000_00000000_00000000) >> ((8 * 3) + 4)) != 0)]
    #[bw(ignore)]
    pub fx1_control: bool,

    // fx1 deep (0: off, 1: on)
    #[br(calc = ((settings & 0b00001000_00000000_00000000_00000000) >> ((8 * 3) + 3)) != 0)]
    #[bw(ignore)]
    pub fx2_deep: bool,

    /// Which part the equalizer applies to: `0` lower, `1` upper, `2` lower+upper.
    ///
    /// Whether the EQ is engaged at all is a separate bit,
    /// [`EffectsPanel::equalizer_on`] — so `0` here means *lower*, not *off*. Located
    /// by diffing the `equalizer/{0,1,2,3}_…` specimens, which differ only at `0xa1`
    /// (and in the enable bit and CRC).
    #[br(calc = ((settings & 0b00000110_00000000_00000000_00000000) >> ((8 * 3) + 1)) as u8)]
    #[bw(ignore)]
    pub equalizer_part: u8,
}

#[binrw]
#[derive(Debug)]
#[br(little, stream = r, map_stream = CrcReader::new(0x2c, 0xa4 - 0x2c), assert(r.checksum() == crc32, "bad checksum: {:#x?} != {:#x?}", r.checksum(), crc32))]
#[bw(little, stream = w, map_stream = CrcWriter::new(0x2c, 0xa4 - 0x2c))]
pub struct Schema {
    pub header: Header,

    pub version: u32,

    // 0x18..0x1a
    #[bw(try_calc = w.checksum())]
    crc32: u32,

    // 0x2c..0x2d
    #[brw(big, pad_before = 16)]
    program_version: u16,

    // 0x2e..0x34
    pub center_panel: CenterPanel,

    // 0x35..0x3b
    pad1: [u8; (0x39 - 0x34) as usize],

    // 0x3a..0x41
    pub piano_panel: PianoPanel,

    // 0x42..0x45
    pad2: [u8; (0x45 - 0x41) as usize],

    // 0x46..0x4d
    pub sample_panel: SamplePanel,

    // 0x4e..0x92
    pub organ_panel: OrganPanel,

    // 0x93..0x9f
    pub effects_panel: EffectsPanel,

    // 0xa0
    todo: u8,

    // 0xa1..0xa4
    pub extra: Extra,
}

#[derive(Debug)]
pub struct Program {
    pub schema: Schema,
    location: Location,
    name: Option<String>,
}

/// Why the accessors below may unwrap a decode. See [`CenterPanel::validate`].
const VALIDATED: &str =
    "centre panel was validated at read and can only be changed through a typed setter";

impl Program {
    pub fn new(location: Location) -> Program {
        Program {
            location,
            name: None,
            schema: Schema {
                header: Header::new(1, FORMAT, location),
                version: 4,
                pad1: [0; (0x39 - 0x34) as usize],
                pad2: [0; (0x45 - 0x41) as usize],
                todo: 0,
                program_version: 4,
                center_panel: CenterPanel::default(),
                piano_panel: PianoPanel::default(),
                sample_panel: SamplePanel::default(),
                organ_panel: OrganPanel {
                    raw: [0; ORGAN_LEN],
                },
                effects_panel: EffectsPanel::default(),
                extra: Extra::default(),
            },
        }
    }

    pub fn read_from(reader: &mut impl BinReaderExt) -> Result<Program, std::io::Error> {
        let schema = match Schema::read_be(reader) {
            Ok(schema) => schema,
            Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
        };
        if !KNOWN_VERSIONS.contains(&schema.version) {
            return Err(io::Error::other(
                crate::error::ParseError::UnsupportedVersion {
                    format: FORMAT,
                    version: schema.version,
                    supported: KNOWN_VERSIONS,
                }
                .to_string(),
            ));
        }

        // Panels holding a private backing word decode on demand rather than at read,
        // so their range checks are made here instead — same contract as the `try_calc`
        // fields they replaced: a file with an impossible value fails the read.
        schema
            .center_panel
            .validate()
            .map_err(|e| io::Error::other(e.to_string()))?;

        Ok(Program {
            location: schema.header.location,
            name: None,
            schema,
        })
    }

    pub fn write_to(&mut self, writer: &mut impl BinWriterExt) -> Result<(), std::io::Error> {
        self.schema.header.location = self.location;

        match writer.write_be(&mut self.schema) {
            Ok(_) => Ok(()),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
        }
    }

    /// The centre panel, whose fields read *and* write — see [`CenterPanel`].
    ///
    /// The `lower_*`/`upper_*` methods below are a naming facade over it (the panel
    /// speaks left/right, the instrument speaks lower/upper); reach for this when you
    /// want to change something.
    pub fn center(&mut self) -> &mut CenterPanel {
        &mut self.schema.center_panel
    }

    // The centre panel's fallible fields were validated by `read_from`, and the only
    // way to change one afterwards is a typed setter, so these cannot see an invalid
    // value. Keeping the panics here rather than in the panel means the accessor's
    // signature stays honest for anyone constructing a panel by other means.
    pub fn lower_part(&self) -> Instrument {
        self.schema.center_panel.left_part().expect(VALIDATED)
    }

    pub fn upper_part(&self) -> Instrument {
        self.schema.center_panel.right_part().expect(VALIDATED)
    }

    pub fn lower_octave_shift(&self) -> OctaveShift {
        self.schema
            .center_panel
            .left_octave_shift()
            .expect(VALIDATED)
    }

    pub fn upper_octave_shift(&self) -> OctaveShift {
        self.schema
            .center_panel
            .right_octave_shift()
            .expect(VALIDATED)
    }

    pub fn lower_sustain(&self) -> bool {
        self.schema.center_panel.left_sustain()
    }

    pub fn upper_sustain(&self) -> bool {
        self.schema.center_panel.right_sustain()
    }

    pub fn lower_control(&self) -> bool {
        self.schema.center_panel.left_control()
    }

    pub fn upper_control(&self) -> bool {
        self.schema.center_panel.right_control()
    }

    pub fn split_point(&self) -> SplitPoint {
        self.schema.center_panel.split_point().expect(VALIDATED)
    }

    pub fn split(&self) -> bool {
        self.schema.center_panel.split()
    }

    pub fn transpose(&self) -> Transpose {
        self.schema.center_panel.transpose().expect(VALIDATED)
    }

    pub fn transpose_enabled(&self) -> bool {
        self.schema.center_panel.transpose_enabled()
    }

    pub fn part_mix(&self) -> PartMix {
        self.schema.center_panel.part_mix().expect(VALIDATED)
    }

    pub fn gain(&self) -> u8 {
        self.schema.center_panel.gain()
    }

    pub fn fx_panel(&self) -> &EffectsPanel {
        &self.schema.effects_panel
    }

    pub fn extra(&self) -> &Extra {
        &self.schema.extra
    }

    pub fn organ(&self) -> &OrganPanel {
        &self.schema.organ_panel
    }

    /// Which organ the program has selected: `0` b3, `1` b3+bass, `2` pipe, `3` vox,
    /// `4` farfisa. Ordering is from the named specimens in
    /// `nord-corpus/ne5/programs/organ/` (the type-B convention).
    ///
    /// **b3+bass is a selection, not a fifth model.** It shares the B3's storage, but
    /// its two presets are different instruments: preset 1 is the bass manual, where
    /// only drawbars 1–2 do anything and they live outside the nine-nibble block (see
    /// [`OrganPanel::b3_bass_drawbars`]); preset 2 is an ordinary B3. Reading preset 1's
    /// nine nibbles in that mode shows stale values.
    pub fn organ_type(&self) -> u8 {
        self.schema.center_panel.organ_type()
    }

    /// The piano panel, including [`PianoPanel::id`] — the program's **piano dependency
    /// reference**. It is the same id the instrument reports for this program over USB
    /// in a `DEPENDENCIES` reply, which is what lets a file on disk be matched to the
    /// library content it needs. The wire carries the piano's *name* too; the file does
    /// not, so resolving one to the other needs the device or a bundle manifest.
    pub fn piano(&self) -> &PianoPanel {
        &self.schema.piano_panel
    }

    /// The sample panel, including [`SamplePanel::id`] — the sample dependency
    /// reference, the counterpart to [`Program::piano`].
    pub fn sample(&self) -> &SamplePanel {
        &self.schema.sample_panel
    }
}

impl bank::Item<Location> for Program {
    fn name(&self) -> Option<String> {
        self.name.clone()
    }

    fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    fn location(&self) -> Location {
        self.location
    }

    fn set_location(&mut self, location: Location) {
        self.location = location;
    }
}

impl common::program::Program for Program {}

#[cfg(test)]
mod tests {
    use super::*;

    /// An unknown schema version is refused at read, not decoded on a guess.
    ///
    /// Field offsets are only validated for the versions in the corpus. A future
    /// firmware bumping `ne5p` to 5 could move fields; decoding it with version-4
    /// offsets would yield plausible but wrong values, and writing it back would then
    /// persist them. Refusing is the only safe default.
    #[test]
    fn an_unknown_schema_version_is_refused() {
        use std::io::Cursor;

        let mut program = Program::new((0, 0).try_into().unwrap());
        let mut bytes = Vec::new();
        program.write_to(&mut Cursor::new(&mut bytes)).unwrap();
        assert_eq!(bytes.len(), FILE_LEN);

        // Sanity: as written, it reads back.
        assert!(Program::read_from(&mut Cursor::new(&mut bytes.clone())).is_ok());

        // The schema version lives at 0x14, little-endian.
        assert_eq!(u32::from_le_bytes(bytes[0x14..0x18].try_into().unwrap()), 4);
        bytes[0x14..0x18].copy_from_slice(&5u32.to_le_bytes());

        let err = Program::read_from(&mut Cursor::new(&mut bytes))
            .expect_err("version 5 must not decode");
        assert!(
            err.to_string().contains("not supported"),
            "unhelpful error: {err}",
        );
    }

    /// Build an organ panel from `(absolute offset, byte)` pairs; everything else 0.
    fn panel(bytes: &[(usize, u8)]) -> OrganPanel {
        let mut raw = [0u8; ORGAN_LEN];
        for &(at, b) in bytes {
            raw[org(at)] = b;
        }
        OrganPanel { raw }
    }

    /// The bass drawbars of b3+bass preset 1 live outside the nine-nibble block, in a
    /// 12-bit field across `0x59`'s low nibble and `0x5a`. Values are from real
    /// specimens: `1100_400000000`, `1100_040000000`, `1100_87gfedcba`.
    #[test]
    fn b3_bass_drawbars_decode_from_the_packed_field() {
        // bar1 = 4, bar2 = 0  ->  field 0x100
        assert_eq!(
            panel(&[(0x59, 0x01), (0x5a, 0x00)]).b3_bass_drawbars(),
            [4, 0]
        );
        // bar1 = 0, bar2 = 4  ->  field 0x010
        assert_eq!(
            panel(&[(0x59, 0x00), (0x5a, 0x10)]).b3_bass_drawbars(),
            [0, 4]
        );
        // bar1 = 8, bar2 = 7  ->  field 0x21c
        assert_eq!(
            panel(&[(0x59, 0x02), (0x5a, 0x1c)]).b3_bass_drawbars(),
            [8, 7]
        );
        // bar1 = 8, bar2 = 8  ->  field 0x220
        assert_eq!(
            panel(&[(0x59, 0x02), (0x5a, 0x20)]).b3_bass_drawbars(),
            [8, 8]
        );
        // all down
        assert_eq!(panel(&[]).b3_bass_drawbars(), [0, 0]);
    }

    /// `0x59` is shared. Its high nibble is bar 9 of the main block and bits 3/2 are
    /// vibrato/percussion — none of which may leak into the bass drawbars. Regression
    /// guard for the `& 0xF` masks.
    #[test]
    fn b3_bass_drawbars_ignore_the_flags_sharing_that_byte() {
        // Same field as `1100_88iiiiiii`: bar 9 = 8 in the high nibble, bars still 8,8.
        assert_eq!(
            panel(&[(0x59, 0x82), (0x5a, 0x20)]).b3_bass_drawbars(),
            [8, 8]
        );
        // Vibrato (0x08) and percussion (0x04) on must not disturb the reading.
        assert_eq!(
            panel(&[(0x59, 0x0e), (0x5a, 0x20)]).b3_bass_drawbars(),
            [8, 8]
        );
        assert_eq!(
            panel(&[(0x59, 0xfe), (0x5a, 0x20)]).b3_bass_drawbars(),
            [8, 8]
        );
    }

    /// Farfisa's drawbars are on/off tabs: >= 5 is on. The raw nibble is still stored
    /// faithfully, it just carries no meaning beyond the threshold.
    #[test]
    fn farfisa_drawbars_are_on_off_tabs() {
        // 0x77 is Farfisa preset 1: nine nibbles, high-nibble first.
        // bars 0..8 = 8,7,6,5,4,3,2,1,0 -> on for >= 5.
        let p = panel(&[
            (0x77, 0x87),
            (0x78, 0x65),
            (0x79, 0x43),
            (0x7a, 0x21),
            (0x7b, 0x00),
        ]);
        assert_eq!(
            p.drawbars(OrganModel::Farfisa, 1),
            [8, 7, 6, 5, 4, 3, 2, 1, 0]
        );
        assert_eq!(
            p.farfisa_tabs(1),
            [true, true, true, true, false, false, false, false, false]
        );
        // The threshold sits between 4 and 5.
        let edge = panel(&[
            (0x77, 0x54),
            (0x78, 0x00),
            (0x79, 0x00),
            (0x7a, 0x00),
            (0x7b, 0x00),
        ]);
        assert_eq!(edge.farfisa_tabs(1)[0], true, "5 should read as on");
        assert_eq!(edge.farfisa_tabs(1)[1], false, "4 should read as off");
    }

    /// Preset 2 reads from its own block, so the two presets never alias.
    #[test]
    fn farfisa_presets_are_independent() {
        let p = panel(&[(0x77, 0x80), (0x7d, 0x08)]);
        assert_eq!(p.farfisa_tabs(1)[0], true);
        assert_eq!(p.farfisa_tabs(1)[1], false);
        assert_eq!(p.farfisa_tabs(2)[0], false);
        assert_eq!(p.farfisa_tabs(2)[1], true);
    }

    // ── centre panel: the RFC-0001 acceptance tests ──────────────────────────────
    //
    // RFC-0002 P0.4 asks for one mutation test per panel: set a field, write, read
    // back, assert the value changed *and* that nothing else did. Every one of these
    // failed before the B+ conversion — assigning to a decoded field compiled, changed
    // nothing, and reported success.

    use std::io::Cursor;

    /// A panel with nothing at a default, so a mutation that leaks into a neighbour has
    /// something to disturb.
    fn busy_panel() -> CenterPanel {
        let mut p = CenterPanel::default();
        p.set_left_part(Instrument::Sample);
        p.set_right_part(Instrument::Piano);
        p.set_left_octave_shift((-6i8).try_into().unwrap());
        p.set_right_octave_shift(5i8.try_into().unwrap());
        p.set_left_sustain(true);
        p.set_right_sustain(true);
        p.set_left_control(true);
        p.set_right_control(true);
        p.set_split(true);
        p.set_split_point(SplitPoint::F5);
        p.set_transpose_enabled(true);
        p.set_transpose(4i8.try_into().unwrap());
        p.set_part_mix(100u8.try_into().unwrap());
        p.set_gain(96).unwrap();
        p.set_organ_type(4).unwrap();
        p.set_lower_enabled(true);
        p.set_upper_enabled(true);
        p.set_drawbar_live(true);
        p
    }

    /// Every decoded value of the panel, rendered for comparison. Keeping this in one
    /// place is what makes "and nothing else moved" checkable field by field.
    fn decoded(p: &CenterPanel) -> Vec<(&'static str, String)> {
        vec![
            ("left_part", format!("{:?}", p.left_part())),
            ("right_part", format!("{:?}", p.right_part())),
            ("left_octave_shift", format!("{:?}", p.left_octave_shift())),
            (
                "right_octave_shift",
                format!("{:?}", p.right_octave_shift()),
            ),
            ("left_sustain", format!("{:?}", p.left_sustain())),
            ("right_sustain", format!("{:?}", p.right_sustain())),
            ("left_control", format!("{:?}", p.left_control())),
            ("right_control", format!("{:?}", p.right_control())),
            ("unknown_boolean1", format!("{:?}", p.unknown_boolean1())),
            ("split", format!("{:?}", p.split())),
            ("split_point", format!("{:?}", p.split_point())),
            ("transpose_enabled", format!("{:?}", p.transpose_enabled())),
            ("transpose", format!("{:?}", p.transpose())),
            ("part_mix", format!("{:?}", p.part_mix())),
            ("gain", format!("{:?}", p.gain())),
            ("organ_type", format!("{:?}", p.organ_type())),
            ("lower_enabled", format!("{:?}", p.lower_enabled())),
            ("upper_enabled", format!("{:?}", p.upper_enabled())),
            ("drawbar_live", format!("{:?}", p.drawbar_live())),
        ]
    }

    /// Each setter, paired with the field it is allowed to change.
    #[allow(clippy::type_complexity)]
    fn mutations() -> Vec<(&'static str, fn(&mut CenterPanel))> {
        vec![
            ("left_part", |p| p.set_left_part(Instrument::Organ)),
            ("right_part", |p| p.set_right_part(Instrument::Sample)),
            ("left_octave_shift", |p| {
                p.set_left_octave_shift(6i8.try_into().unwrap())
            }),
            ("right_octave_shift", |p| {
                p.set_right_octave_shift((-3i8).try_into().unwrap())
            }),
            ("left_sustain", |p| p.set_left_sustain(false)),
            ("right_sustain", |p| p.set_right_sustain(false)),
            ("left_control", |p| p.set_left_control(false)),
            ("right_control", |p| p.set_right_control(false)),
            ("split", |p| p.set_split(false)),
            ("split_point", |p| p.set_split_point(SplitPoint::Lower)),
            ("transpose_enabled", |p| p.set_transpose_enabled(false)),
            ("transpose", |p| p.set_transpose((-5i8).try_into().unwrap())),
            ("part_mix", |p| p.set_part_mix(13u8.try_into().unwrap())),
            ("gain", |p| p.set_gain(127).unwrap()),
            ("organ_type", |p| p.set_organ_type(1).unwrap()),
            ("lower_enabled", |p| p.set_lower_enabled(false)),
            ("upper_enabled", |p| p.set_upper_enabled(false)),
            ("drawbar_live", |p| p.set_drawbar_live(false)),
        ]
    }

    /// The headline defect RFC-0001 exists to fix: a write to a decoded field used to
    /// be discarded silently. Every setter must move its own field and no other.
    #[test]
    fn setting_a_field_changes_that_field_and_only_that_field() {
        for (name, mutate) in mutations() {
            let before = busy_panel();
            let mut after = busy_panel();
            mutate(&mut after);

            for ((field, was), (_, now)) in decoded(&before).into_iter().zip(decoded(&after)) {
                if field == name {
                    assert_ne!(was, now, "set_{name} did not change {field}");
                } else {
                    assert_eq!(was, now, "set_{name} disturbed {field}");
                }
            }
        }
    }

    /// …and it has to survive the trip through the bytes, which is the half that
    /// `#[bw(ignore)]` used to drop.
    #[test]
    fn a_mutated_field_survives_a_write_read_cycle() {
        for (name, mutate) in mutations() {
            let mut program = Program::new((0, 0).try_into().unwrap());
            *program.center() = busy_panel();
            mutate(program.center());
            let expected = decoded(program.center());

            let mut bytes = Vec::new();
            program.write_to(&mut Cursor::new(&mut bytes)).unwrap();
            let reread = Program::read_from(&mut Cursor::new(&mut bytes)).unwrap_or_else(|e| {
                panic!("set_{name} produced a program that will not read: {e}")
            });

            assert_eq!(
                expected,
                decoded(&reread.schema.center_panel),
                "set_{name} did not survive the write",
            );
        }
    }

    /// A centre-panel write must stay inside the centre panel. Only `0x2e..=0x34` and
    /// the body CRC at `0x18..=0x1b` may move.
    #[test]
    fn a_mutation_touches_only_the_panels_own_bytes() {
        let bytes_of = |panel: CenterPanel| {
            let mut program = Program::new((0, 0).try_into().unwrap());
            *program.center() = panel;
            let mut bytes = Vec::new();
            program.write_to(&mut Cursor::new(&mut bytes)).unwrap();
            bytes
        };

        let before = bytes_of(busy_panel());

        for (name, mutate) in mutations() {
            let mut panel = busy_panel();
            mutate(&mut panel);
            let after = bytes_of(panel);

            assert_eq!(before.len(), after.len());
            for (at, (b, a)) in before.iter().zip(&after).enumerate() {
                let allowed = (0x2e..=0x34).contains(&at) || (0x18..=0x1b).contains(&at);
                assert!(
                    allowed || b == a,
                    "set_{name} changed byte {at:#04x} ({b:#04x} -> {a:#04x})",
                );
            }
            assert_ne!(before, after, "set_{name} changed no bytes at all");
        }
    }

    /// Width validation, the requirement that only appears once writes are derived: a
    /// `u8` gain does not fit its seven-bit slot, so an over-wide value has to be
    /// refused rather than allowed to overrun `part_mix` next door.
    #[test]
    fn a_value_too_wide_for_its_slot_is_refused_not_truncated() {
        let mut panel = busy_panel();
        let before = decoded(&panel);

        let err = panel.set_gain(128).unwrap_err();
        assert_eq!(err.width, 7);
        assert_eq!(
            decoded(&panel),
            before,
            "a refused write still changed bits"
        );

        // Three bits for the organ selection; 8 is one too many.
        assert!(panel.set_organ_type(8).is_err());
        assert_eq!(
            decoded(&panel),
            before,
            "a refused write still changed bits"
        );

        // The largest value that does fit still goes through.
        assert!(panel.set_organ_type(7).is_ok());
        assert_eq!(panel.organ_type(), 7);
        assert_eq!(panel.gain(), 96, "a legal write disturbed its neighbour");
    }

    /// Rewriting a field with the value just read must be a no-op, on any word — the
    /// property that says a setter lands exactly where its getter looks. The gap bits
    /// (`settings3` 7..0, which no field names) are what makes this non-trivial.
    #[test]
    fn rewriting_a_field_with_its_own_value_changes_nothing() {
        for pattern in [0x0000_0000u32, 0xffff_ffff, 0xa5a5_a5a5, 0x5a5a_5a5a] {
            let mut panel = CenterPanel {
                settings: pattern as u16,
                settings2: pattern as u8,
                settings3: pattern,
            };
            // Only fields whose current bits decode can be written back.
            if panel.validate().is_err() {
                continue;
            }
            let (s1, s2, s3) = (panel.settings, panel.settings2, panel.settings3);

            panel.set_left_part(panel.left_part().unwrap());
            panel.set_right_part(panel.right_part().unwrap());
            panel.set_left_octave_shift(panel.left_octave_shift().unwrap());
            panel.set_right_octave_shift(panel.right_octave_shift().unwrap());
            panel.set_left_sustain(panel.left_sustain());
            panel.set_right_sustain(panel.right_sustain());
            panel.set_left_control(panel.left_control());
            panel.set_right_control(panel.right_control());
            panel.set_split(panel.split());
            panel.set_split_point(panel.split_point().unwrap());
            panel.set_transpose_enabled(panel.transpose_enabled());
            panel.set_transpose(panel.transpose().unwrap());
            panel.set_part_mix(panel.part_mix().unwrap());
            panel.set_gain(panel.gain()).unwrap();
            panel.set_organ_type(panel.organ_type()).unwrap();
            panel.set_lower_enabled(panel.lower_enabled());
            panel.set_upper_enabled(panel.upper_enabled());
            panel.set_drawbar_live(panel.drawbar_live());

            assert_eq!(
                (s1, s2, s3),
                (panel.settings, panel.settings2, panel.settings3)
            );
        }
    }

    /// The default panel has to decode, which a zeroed word does not: an octave shift
    /// of zero is stored as 7, so all-zero bits mean `-7` — out of range. The old
    /// `Default` got away with it because nothing ever read it back.
    #[test]
    fn the_default_panel_decodes() {
        let panel = CenterPanel::default();
        panel.validate().expect("default panel must decode");

        assert_eq!(panel.left_octave_shift().unwrap(), 0);
        assert_eq!(panel.right_octave_shift().unwrap(), 0);
        assert_eq!(panel.transpose().unwrap(), 0);
        assert_eq!(panel.left_part().unwrap(), Instrument::Organ);
    }

    /// A file whose bits do not decode is still refused at read, not at access — the
    /// contract `try_calc` used to provide, now enforced by `validate`.
    #[test]
    fn a_program_whose_center_panel_cannot_decode_is_refused_at_read() {
        let mut program = Program::new((0, 0).try_into().unwrap());
        // 0b111 is not an instrument, and it is not reachable through the setters.
        program.schema.center_panel.settings |= 0b1110_0000_0000_0000;

        let mut bytes = Vec::new();
        program.write_to(&mut Cursor::new(&mut bytes)).unwrap();

        let err = Program::read_from(&mut Cursor::new(&mut bytes))
            .expect_err("an undecodable centre panel must not read");
        assert!(
            err.to_string().contains("exceeds bound"),
            "unhelpful: {err}"
        );
    }
}
