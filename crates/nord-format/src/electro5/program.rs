use crate::bits::{Field, FieldOverflow, Straddle};
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

// 0x3a..0x41 — the piano panel. B+ as above: one private word, one `Field` per value.
//
// Every field here is a raw integer or a flag, so the getters are infallible; only the
// narrow slots need a checked setter. Bits 60..59 and 53..49 are named by nothing —
// they are the gap bits RFC-0001 calls out, and under B+ they need no declaration at
// all.

/// 0x3a..0x41 — `settings`.
type PianoCategory = Field<u8, 63, 61>;
type PianoModel = Field<u8, 58, 54>;
type ClavModel = Field<u8, 48, 47>;
type PianoAcoustics = Field<u8, 46, 45>;
type PianoTouch = Field<u8, 44, 43>;
type PianoMono = Field<bool, 42, 42>;
type PianoId = Field<u32, 41, 10>;

#[binrw]
#[derive(Default)]
pub struct PianoPanel {
    // 0x3a..0x41               0x3a      0x3b     0x3c     0x3d     0x3e     0x3f     0x40    0x41
    #[brw(big)]
    settings: u64,
}

impl PianoPanel {
    /// 5 == 0, 6 == 1, 1 == 2, 2 == 3, 3 == 4, 4 == 5.
    pub fn category(&self) -> u8 {
        PianoCategory::read(self.settings)
    }

    pub fn set_category(&mut self, value: u8) -> Result<(), FieldOverflow> {
        PianoCategory::checked_set(&mut self.settings, value)
    }

    /// Zero-based model slot *within* [`category`](Self::category) — the panel's
    /// Model dial. A slot coordinate, not an identity; see [`id`](Self::id).
    pub fn piano_model(&self) -> u8 {
        PianoModel::read(self.settings)
    }

    pub fn set_piano_model(&mut self, value: u8) -> Result<(), FieldOverflow> {
        PianoModel::checked_set(&mut self.settings, value)
    }

    pub fn clav_model(&self) -> u8 {
        ClavModel::read(self.settings)
    }

    pub fn set_clav_model(&mut self, value: u8) -> Result<(), FieldOverflow> {
        ClavModel::checked_set(&mut self.settings, value)
    }

    pub fn acoustics(&self) -> u8 {
        PianoAcoustics::read(self.settings)
    }

    pub fn set_acoustics(&mut self, value: u8) -> Result<(), FieldOverflow> {
        PianoAcoustics::checked_set(&mut self.settings, value)
    }

    pub fn touch(&self) -> u8 {
        PianoTouch::read(self.settings)
    }

    pub fn set_touch(&mut self, value: u8) -> Result<(), FieldOverflow> {
        PianoTouch::checked_set(&mut self.settings, value)
    }

    pub fn mono(&self) -> bool {
        PianoMono::read(self.settings)
    }

    pub fn set_mono(&mut self, value: bool) {
        PianoMono::set(&mut self.settings, value)
    }

    /// The piano (`.npno`) this program depends on: a stable 32-bit id in bits
    /// 41..=10, independent of where the piano sits in the instrument's library.
    /// `0` means "no piano referenced". This — not
    /// [`category`](Self::category)/[`piano_model`](Self::piano_model), which are
    /// slot coordinates — is what resolves the song → program → piano chain, and
    /// what Nord Sound Manager checks to decide whether a Restore is missing a
    /// dependency.
    pub fn id(&self) -> u32 {
        PianoId::read(self.settings)
    }

    /// Infallible, unlike its neighbours: the slot is a full 32 bits, so no `u32` can
    /// overrun it and the fit is proven at compile time.
    pub fn set_id(&mut self, value: u32) {
        PianoId::set(&mut self.settings, value)
    }
}

impl Debug for PianoPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PianoPanel")
            .field("category", &self.category())
            .field("piano_model", &self.piano_model())
            .field("clav_model", &self.clav_model())
            .field("acoustics", &self.acoustics())
            .field("touch", &self.touch())
            .field("mono", &self.mono())
            .field("id", &self.id())
            .finish()
    }
}

// 0x46..0x4d — the sample panel.

/// 0x46..0x4d — `settings`.
type SampleAttack = Field<u8, 63, 57>;
type SampleDecayRelease = Field<u8, 56, 50>;
type SampleNumber = Field<u8, 49, 42>;
type SampleId = Field<u32, 41, 10>;
type SampleDynamics = Field<u8, 9, 8>;
type SampleFilter = Field<bool, 7, 7>;

#[binrw]
#[derive(Default)]
pub struct SamplePanel {
    // 0x46..0x4d               0x46      0x47     0x48     0x49     0x4a     0x4b     0x4c    0x4d
    #[brw(big)]
    settings: u64,
}

impl SamplePanel {
    pub fn attack(&self) -> u8 {
        SampleAttack::read(self.settings)
    }

    pub fn set_attack(&mut self, value: u8) -> Result<(), FieldOverflow> {
        SampleAttack::checked_set(&mut self.settings, value)
    }

    pub fn decay_release(&self) -> u8 {
        SampleDecayRelease::read(self.settings)
    }

    pub fn set_decay_release(&mut self, value: u8) -> Result<(), FieldOverflow> {
        SampleDecayRelease::checked_set(&mut self.settings, value)
    }

    /// Zero-based slot of the sample in the instrument's Samp Lib, i.e. the
    /// number shown on the panel minus one. This is a *position*, not an
    /// identity: adding or deleting samples renumbers it, and the corpus has
    /// ids that appear under several numbers (and numbers reused by several
    /// ids). Use [`id`](Self::id) to resolve the dependency.
    ///
    /// The slot is a full eight bits, so this setter is infallible.
    pub fn number(&self) -> u8 {
        SampleNumber::read(self.settings)
    }

    pub fn set_number(&mut self, value: u8) {
        SampleNumber::set(&mut self.settings, value)
    }

    /// The sample (`.nsmp`) this program depends on: a stable 32-bit id in bits
    /// 41..=10, laid out exactly as [`PianoPanel::id`]. `0` means "no sample
    /// referenced".
    ///
    /// Same bit range, same type, same `Field` — the first shared component, reused
    /// across two panels before a second device model exists to reuse it across.
    pub fn id(&self) -> u32 {
        SampleId::read(self.settings)
    }

    pub fn set_id(&mut self, value: u32) {
        SampleId::set(&mut self.settings, value)
    }

    pub fn dynamics(&self) -> u8 {
        SampleDynamics::read(self.settings)
    }

    pub fn set_dynamics(&mut self, value: u8) -> Result<(), FieldOverflow> {
        SampleDynamics::checked_set(&mut self.settings, value)
    }

    pub fn filter(&self) -> bool {
        SampleFilter::read(self.settings)
    }

    pub fn set_filter(&mut self, value: bool) {
        SampleFilter::set(&mut self.settings, value)
    }
}

impl Debug for SamplePanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SamplePanel")
            .field("attack", &self.attack())
            .field("decay_release", &self.decay_release())
            .field("number", &self.number())
            .field("id", &self.id())
            .field("dynamics", &self.dynamics())
            .field("filter", &self.filter())
            .finish()
    }
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

// The organ panel's flags, as positions within whichever byte holds them. Unlike the
// other panels these aliases are not bound to one word — the same `VibOn` is applied to
// four different bytes — so nothing stops the wrong alias being applied to the wrong
// byte. That is the cost of `Field` over a byte array rather than a backing word, and
// the reason RFC-0001 treats a full `OrganPanel` conversion as its own question.

/// `0x53`/`0x65`/`0x75`/`0x85` bit 6 — preset 2 selected.
type OrganPreset = Field<bool, 6, 6>;
/// Nibble-packed drawbars, high nibble first.
type DrawbarHigh = Field<u8, 7, 4>;
type DrawbarLow = Field<u8, 3, 0>;
/// `0x59`/`0x60`/`0x6b`/`0x71`/`0x7b`/`0x81` bit 3 — vibrato/chorus on.
type VibOn = Field<bool, 3, 3>;
/// `0x51`/`0x63`/`0x73` bits 7..5 — index into the model's vib/chorus table.
type VibType = Field<u8, 7, 5>;
/// `0x59`/`0x60` bit 2 — B3 percussion on.
type PercOn = Field<bool, 2, 2>;
/// `0x51` bit 4 — percussion third harmonic.
type PercThird = Field<bool, 4, 4>;
/// `0x51` bits 3..2 — percussion decay speed, encoded 2/1/3 for soft/fast/both.
type PercSpeedField = Field<u8, 3, 2>;
/// Within the 12-bit b3-bass field assembled from `0x59`'s low nibble and `0x5a`.
type BassBar1 = Field<u8, 9, 6>;
type BassBar2 = Field<u8, 5, 2>;
/// `0x59`'s low nibble — the only part of that byte the b3-bass field owns.
type BassHighNibble = Field<u8, 3, 0>;

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

/// `[u8; 69]` has no `Default` — the std impls stop at 32 — so this one is written out
/// rather than derived like the other panels'.
impl Default for OrganPanel {
    fn default() -> Self {
        OrganPanel {
            raw: [0; ORGAN_LEN],
        }
    }
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
        let (byte, table) = Self::vib_table(model)?;
        table.get(VibType::read(self.raw[byte]) as usize).copied()
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

    // ── writes ──────────────────────────────────────────────────────────────────
    //
    // The organ panel keeps its existing shape rather than converting to `Field`:
    // RFC-0001 notes it is the best-tested and most-correct code in the file, and its
    // 552 bits live in a `[u8; 69]`, which is not a backing *word*. Its problem was
    // never silent write loss — it has no public decoded fields to assign to — but the
    // absence of any way to write at all. `Field` still does the per-byte bit work, so
    // the position of each flag is authored once here too.

    /// Select `preset` (1 or 2) for `model`.
    pub fn set_preset(&mut self, model: OrganModel, preset: u8) {
        OrganPreset::set(&mut self.raw[Self::preset_byte(model)], preset == 2);
    }

    /// Store nine drawbar positions for `model`'s `preset`. Positions are physical,
    /// `0..=8`; anything higher is refused rather than truncated into its neighbour,
    /// since two bars share a byte.
    pub fn set_drawbars(
        &mut self,
        model: OrganModel,
        preset: u8,
        bars: [u8; 9],
    ) -> Result<(), FieldOverflow> {
        if let Some(&bad) = bars.iter().find(|&&b| b > 8) {
            return Err(FieldOverflow {
                value: bad as u64,
                width: 4,
            });
        }

        let at = Self::drawbar_offset(model, preset);
        for (n, bar) in bars.into_iter().enumerate() {
            let byte = &mut self.raw[at + n / 2];
            if n % 2 == 0 {
                DrawbarHigh::checked_set(byte, bar)?;
            } else {
                DrawbarLow::checked_set(byte, bar)?;
            }
        }
        Ok(())
    }

    /// Set the Farfisa tabs, which are on/off rather than continuous: on stores `8`,
    /// off stores `0`. Any other stored value that happened to be there is lost — the
    /// instrument only reads which side of the ≥5 threshold it falls on, but the byte
    /// itself does change, so this is not a round-trip-safe way to touch a Farfisa
    /// program you did not mean to rewrite.
    pub fn set_farfisa_tabs(&mut self, preset: u8, tabs: [bool; 9]) {
        let mut bars = [0u8; 9];
        for (bar, on) in bars.iter_mut().zip(tabs) {
            *bar = if on { 8 } else { 0 };
        }
        self.set_drawbars(OrganModel::Farfisa, preset, bars)
            .expect("0 and 8 are both in range");
    }

    /// Turn vibrato/chorus on or off for `model`'s `preset`. No-op for Pipe, which has
    /// none.
    pub fn set_vib_on(&mut self, model: OrganModel, preset: u8, on: bool) {
        if let Some(i) = Self::effect_byte(model, preset) {
            VibOn::set(&mut self.raw[i], on);
        }
    }

    /// Select the vibrato/chorus mode for `model` (shared across presets).
    ///
    /// Fails when the mode is not one the model offers — each exposes a different
    /// subset of the six, and Pipe none at all. The stored value is the *index* into
    /// that model's table, so this cannot be a plain bit write.
    pub fn set_vib_type(&mut self, model: OrganModel, vib: VibChorus) -> Result<(), ParseError> {
        let (byte, table) = match Self::vib_table(model) {
            Some(pair) => pair,
            None => {
                return Err(ParseError::OutOfBounds(
                    format!("{vib:?}"),
                    format!("{model:?} has no vibrato/chorus"),
                ))
            }
        };

        match table.iter().position(|&v| v == vib) {
            Some(index) => {
                VibType::checked_set(&mut self.raw[byte], index as u8)
                    .expect("no model's table has more than six entries");
                Ok(())
            }
            None => Err(ParseError::OutOfBounds(
                format!("{vib:?}"),
                format!("{model:?} offers {table:?}"),
            )),
        }
    }

    /// Turn B3 percussion on or off for `preset`.
    pub fn set_b3_perc_on(&mut self, preset: u8, on: bool) {
        let at = org(if preset == 2 { 0x60 } else { 0x59 });
        PercOn::set(&mut self.raw[at], on);
    }

    /// Percussion third harmonic (shared across presets).
    pub fn set_b3_perc_third(&mut self, on: bool) {
        PercThird::set(&mut self.raw[org(0x51)], on);
    }

    /// Percussion decay speed (shared across presets). Note the encoding is not
    /// monotonic — see [`Self::b3_perc_speed`].
    pub fn set_b3_perc_speed(&mut self, speed: PercSpeed) {
        let bits = match speed {
            PercSpeed::Off => 0,
            PercSpeed::Soft => 2,
            PercSpeed::Fast => 1,
            PercSpeed::Both => 3,
        };
        PercSpeedField::checked_set(&mut self.raw[org(0x51)], bits)
            .expect("all four speeds encode in two bits");
    }

    /// The two bass drawbars of B3-with-bass preset 1 — a 12-bit field spanning
    /// `0x59`'s low nibble and `0x5a`, so a straddler inside a byte array rather than
    /// across two words. Positions are `0..=8`; the field's low two bits are unused and
    /// are left as they are found.
    pub fn set_b3_bass_drawbars(&mut self, bars: [u8; 2]) -> Result<(), FieldOverflow> {
        if let Some(&bad) = bars.iter().find(|&&b| b > 8) {
            return Err(FieldOverflow {
                value: bad as u64,
                width: 4,
            });
        }

        let mut field = ((self.raw[org(0x59)] as u16 & 0x0F) << 8) | self.raw[org(0x5a)] as u16;
        BassBar1::checked_set(&mut field, bars[0])?;
        BassBar2::checked_set(&mut field, bars[1])?;

        // Only the low nibble of 0x59 belongs to the field; its high nibble is bar 9 of
        // the main block and bits 3/2 are vibrato and percussion.
        BassHighNibble::checked_set(&mut self.raw[org(0x59)], (field >> 8) as u8)
            .expect("the field was assembled from a nibble, so bits 15..12 are clear");
        self.raw[org(0x5a)] = field as u8;
        Ok(())
    }

    /// The vib/chorus selection byte and the modes it indexes, or `None` for Pipe.
    fn vib_table(model: OrganModel) -> Option<(usize, &'static [VibChorus])> {
        use VibChorus::*;
        match model {
            OrganModel::B3 => Some((org(0x51), &[V1, C1, V2, C2, V3, C3])),
            OrganModel::Vox => Some((org(0x63), &[V1, V2, V3])),
            OrganModel::Farfisa => Some((org(0x73), &[V1, V2, C2, C3])),
            OrganModel::Pipe => None,
        }
    }
}

// 0x93..0x9f — the effects panel, and the interesting one for B+: it holds both of the
// format's cross-word fields.
//
// `equalizer_freq_gain` spans `settings`→`settings2` and `fx5_moisture` spans
// `settings2`→`settings3`. RFC-0001 scored B+ as needing a "2-call compose" for these,
// which was the honest reading of the option as written — but a `Straddle` of two
// ranges composes them into one field type, so at the call site they are ordinary
// fields with an ordinary getter and setter. That removes the straddler column as a
// reason to prefer a continuous-cursor option.

/// 0x93..0x9a — `settings`.
type Fx1 = Field<u8, 63, 62>;
type Fx1Type = Field<u8, 61, 58>;
type Fx1Rate = Field<u8, 57, 51>;
type Fx2 = Field<u8, 50, 49>;
type Fx2Type = Field<u8, 48, 45>;
type Fx2Rate = Field<u8, 44, 38>;
type Fx4 = Field<u8, 37, 36>;
type Fx4Feedback = Field<u8, 35, 34>;
type Fx4Tempo = Field<u8, 33, 27>;
type Fx4Moisture = Field<u8, 26, 20>;
type Fx4PingPong = Field<bool, 19, 19>;
type EqualizerOn = Field<bool, 18, 18>;
type EqualizerFreq = Field<u8, 16, 10>;
type EqualizerTreble = Field<u8, 9, 3>;

/// 0x9b..0x9e — `settings2`.
type EqualizerBass = Field<u8, 27, 21>;
type Fx3 = Field<u8, 20, 19>;
type Fx3Type = Field<u8, 18, 16>;
type Fx3Compression = Field<u8, 15, 9>;
type Fx5 = Field<bool, 8, 8>;
type Fx5Type = Field<u8, 7, 5>;

/// 0x9f — `settings3`.
type RotaryStop = Field<u8, 5, 5>;
type RotarySpeed = Field<u8, 4, 4>;

/// The two cross-word fields, each named once as a pair of ranges rather than as a
/// hand-written shift-and-add on the read side and nothing at all on the write side.
type EqualizerFreqGain = Straddle<u8, Field<u8, 2, 0>, Field<u8, 31, 28>>;
type Fx5Moisture = Straddle<u8, Field<u8, 4, 0>, Field<u8, 7, 6>>;

#[binrw]
#[derive(Default)]
pub struct EffectsPanel {
    // 0x93..0x9a               0x93      0x94     0x95     0x96     0x97     0x98     0x99    0x9a
    #[brw(big)]
    settings: u64,

    // 0x9b..0x9e
    #[brw(big)]
    settings2: u32,

    // 0x9f
    #[brw(big)]
    settings3: u8,
}

impl EffectsPanel {
    /// 0: off, 1: lower, 2: upper.
    pub fn fx1(&self) -> u8 {
        Fx1::read(self.settings)
    }

    pub fn set_fx1(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Fx1::checked_set(&mut self.settings, value)
    }

    /// 1: pan1, pan2, pan1&2; 2: wah, rm, trem1, trem2, trem1&2.
    pub fn fx1_type(&self) -> u8 {
        Fx1Type::read(self.settings)
    }

    pub fn set_fx1_type(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Fx1Type::checked_set(&mut self.settings, value)
    }

    /// 0..127, shown as 0..10.
    pub fn fx1_rate(&self) -> u8 {
        Fx1Rate::read(self.settings)
    }

    pub fn set_fx1_rate(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Fx1Rate::checked_set(&mut self.settings, value)
    }

    /// 0: off, 1: lower, 2: upper.
    pub fn fx2(&self) -> u8 {
        Fx2::read(self.settings)
    }

    pub fn set_fx2(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Fx2::checked_set(&mut self.settings, value)
    }

    /// flang, choir1, choir2, vibe, phas1, phas2.
    pub fn fx2_type(&self) -> u8 {
        Fx2Type::read(self.settings)
    }

    pub fn set_fx2_type(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Fx2Type::checked_set(&mut self.settings, value)
    }

    /// 0..127, shown as 0..10.
    pub fn fx2_rate(&self) -> u8 {
        Fx2Rate::read(self.settings)
    }

    pub fn set_fx2_rate(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Fx2Rate::checked_set(&mut self.settings, value)
    }

    /// 0: off, 1: lower, 2: upper.
    pub fn fx4(&self) -> u8 {
        Fx4::read(self.settings)
    }

    pub fn set_fx4(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Fx4::checked_set(&mut self.settings, value)
    }

    pub fn fx4_feedback(&self) -> u8 {
        Fx4Feedback::read(self.settings)
    }

    pub fn set_fx4_feedback(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Fx4Feedback::checked_set(&mut self.settings, value)
    }

    /// 0..127, 750ms..20ms.
    pub fn fx4_tempo(&self) -> u8 {
        Fx4Tempo::read(self.settings)
    }

    pub fn set_fx4_tempo(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Fx4Tempo::checked_set(&mut self.settings, value)
    }

    /// Delay wet/dry, 0..127, shown as 0..10.
    pub fn fx4_moisture(&self) -> u8 {
        Fx4Moisture::read(self.settings)
    }

    pub fn set_fx4_moisture(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Fx4Moisture::checked_set(&mut self.settings, value)
    }

    pub fn fx4_ping_pong(&self) -> bool {
        Fx4PingPong::read(self.settings)
    }

    pub fn set_fx4_ping_pong(&mut self, value: bool) {
        Fx4PingPong::set(&mut self.settings, value)
    }

    /// EQ engaged. **Which part it applies to is not in this word** — see
    /// [`Extra::equalizer_part`].
    ///
    /// This was previously decoded as a two-bit `equalizer_part_select`, which could
    /// only ever answer 0 or 2: diffing the four named `equalizer/{0,1,2,3}_…`
    /// specimens shows they are byte-identical across `0x93..0x9a` apart from this
    /// single bit, and the lower/upper/both choice lives at `0xa1`.
    pub fn equalizer_on(&self) -> bool {
        EqualizerOn::read(self.settings)
    }

    pub fn set_equalizer_on(&mut self, value: bool) {
        EqualizerOn::set(&mut self.settings, value)
    }

    pub fn equalizer_freq(&self) -> u8 {
        EqualizerFreq::read(self.settings)
    }

    pub fn set_equalizer_freq(&mut self, value: u8) -> Result<(), FieldOverflow> {
        EqualizerFreq::checked_set(&mut self.settings, value)
    }

    pub fn equalizer_treble(&self) -> u8 {
        EqualizerTreble::read(self.settings)
    }

    pub fn set_equalizer_treble(&mut self, value: u8) -> Result<(), FieldOverflow> {
        EqualizerTreble::checked_set(&mut self.settings, value)
    }

    /// The first of the format's two cross-word fields: three bits at the bottom of
    /// `settings` (0x9a) and four at the top of `settings2` (0x9b). Reading and writing
    /// it is a single call, exactly like any other field.
    pub fn equalizer_freq_gain(&self) -> u8 {
        EqualizerFreqGain::read(self.settings, self.settings2)
    }

    pub fn set_equalizer_freq_gain(&mut self, value: u8) -> Result<(), FieldOverflow> {
        EqualizerFreqGain::checked_set(&mut self.settings, &mut self.settings2, value)
    }

    pub fn equalizer_bass(&self) -> u8 {
        EqualizerBass::read(self.settings2)
    }

    pub fn set_equalizer_bass(&mut self, value: u8) -> Result<(), FieldOverflow> {
        EqualizerBass::checked_set(&mut self.settings2, value)
    }

    /// 0: off, 1: lower, 2: upper.
    pub fn fx3(&self) -> u8 {
        Fx3::read(self.settings2)
    }

    pub fn set_fx3(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Fx3::checked_set(&mut self.settings2, value)
    }

    /// none, twin, rotary, comp, small, jc.
    pub fn fx3_type(&self) -> u8 {
        Fx3Type::read(self.settings2)
    }

    pub fn set_fx3_type(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Fx3Type::checked_set(&mut self.settings2, value)
    }

    /// 0..127, shown as 0..10.
    pub fn fx3_compression(&self) -> u8 {
        Fx3Compression::read(self.settings2)
    }

    pub fn set_fx3_compression(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Fx3Compression::checked_set(&mut self.settings2, value)
    }

    pub fn fx5(&self) -> bool {
        Fx5::read(self.settings2)
    }

    pub fn set_fx5(&mut self, value: bool) {
        Fx5::set(&mut self.settings2, value)
    }

    pub fn fx5_type(&self) -> u8 {
        Fx5Type::read(self.settings2)
    }

    pub fn set_fx5_type(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Fx5Type::checked_set(&mut self.settings2, value)
    }

    /// The second cross-word field: five bits at the bottom of `settings2` (0x9e) and
    /// two at the top of `settings3` (0x9f).
    pub fn fx5_moisture(&self) -> u8 {
        Fx5Moisture::read(self.settings2, self.settings3)
    }

    pub fn set_fx5_moisture(&mut self, value: u8) -> Result<(), FieldOverflow> {
        Fx5Moisture::checked_set(&mut self.settings2, &mut self.settings3, value)
    }

    /// 0 = off, 1 = on.
    pub fn rotary_stop(&self) -> u8 {
        RotaryStop::read(self.settings3)
    }

    pub fn set_rotary_stop(&mut self, value: u8) -> Result<(), FieldOverflow> {
        RotaryStop::checked_set(&mut self.settings3, value)
    }

    /// 0 = slow, 1 = fast.
    pub fn rotary_speed(&self) -> u8 {
        RotarySpeed::read(self.settings3)
    }

    pub fn set_rotary_speed(&mut self, value: u8) -> Result<(), FieldOverflow> {
        RotarySpeed::checked_set(&mut self.settings3, value)
    }
}

impl Debug for EffectsPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EffectsPanel")
            .field("fx1", &self.fx1())
            .field("fx1_type", &self.fx1_type())
            .field("fx1_rate", &self.fx1_rate())
            .field("fx2", &self.fx2())
            .field("fx2_type", &self.fx2_type())
            .field("fx2_rate", &self.fx2_rate())
            .field("fx3", &self.fx3())
            .field("fx3_type", &self.fx3_type())
            .field("fx3_compression", &self.fx3_compression())
            .field("fx4", &self.fx4())
            .field("fx4_feedback", &self.fx4_feedback())
            .field("fx4_tempo", &self.fx4_tempo())
            .field("fx4_moisture", &self.fx4_moisture())
            .field("fx4_ping_pong", &self.fx4_ping_pong())
            .field("fx5", &self.fx5())
            .field("fx5_type", &self.fx5_type())
            .field("fx5_moisture", &self.fx5_moisture())
            .field("equalizer_on", &self.equalizer_on())
            .field("equalizer_freq", &self.equalizer_freq())
            .field("equalizer_freq_gain", &self.equalizer_freq_gain())
            .field("equalizer_bass", &self.equalizer_bass())
            .field("equalizer_treble", &self.equalizer_treble())
            .field("rotary_stop", &self.rotary_stop())
            .field("rotary_speed", &self.rotary_speed())
            .finish()
    }
}

// 0xa1..0xa4

/// 0xa1..0xa4 — `settings`.
type Fx1Control = Field<bool, 28, 28>;
type Fx2Deep = Field<bool, 27, 27>;
type EqualizerPart = Field<u8, 26, 25>;

#[binrw]
#[derive(Default)]
pub struct Extra {
    #[brw(big)]
    settings: u32,
}

impl Extra {
    /// fx1 control pedal.
    pub fn fx1_control(&self) -> bool {
        Fx1Control::read(self.settings)
    }

    pub fn set_fx1_control(&mut self, value: bool) {
        Fx1Control::set(&mut self.settings, value)
    }

    /// fx2 deep.
    pub fn fx2_deep(&self) -> bool {
        Fx2Deep::read(self.settings)
    }

    pub fn set_fx2_deep(&mut self, value: bool) {
        Fx2Deep::set(&mut self.settings, value)
    }

    /// Which part the equalizer applies to: `0` lower, `1` upper, `2` lower+upper.
    ///
    /// Whether the EQ is engaged at all is a separate bit,
    /// [`EffectsPanel::equalizer_on`] — so `0` here means *lower*, not *off*. Located
    /// by diffing the `equalizer/{0,1,2,3}_…` specimens, which differ only at `0xa1`
    /// (and in the enable bit and CRC).
    pub fn equalizer_part(&self) -> u8 {
        EqualizerPart::read(self.settings)
    }

    pub fn set_equalizer_part(&mut self, value: u8) -> Result<(), FieldOverflow> {
        EqualizerPart::checked_set(&mut self.settings, value)
    }
}

impl Debug for Extra {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Extra")
            .field("fx1_control", &self.fx1_control())
            .field("fx2_deep", &self.fx2_deep())
            .field("equalizer_part", &self.equalizer_part())
            .finish()
    }
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
                organ_panel: OrganPanel::default(),
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

    // ── the other panels ────────────────────────────────────────────────────────
    //
    // Same acceptance test, one panel at a time. `PianoPanel`, `SamplePanel` and
    // `Extra` had *no* writable field before the conversion — every one of their
    // backing words was stored verbatim with every decoded field `#[bw(ignore)]`.

    /// Assert that each mutation moves exactly one entry of `snapshot`.
    #[allow(clippy::type_complexity)]
    fn each_setter_moves_only_its_own_field<P>(
        fresh: fn() -> P,
        snapshot: fn(&P) -> Vec<(&'static str, String)>,
        mutations: Vec<(&'static str, fn(&mut P))>,
    ) {
        for (name, mutate) in mutations {
            let before = fresh();
            let mut after = fresh();
            mutate(&mut after);

            for ((field, was), (_, now)) in snapshot(&before).into_iter().zip(snapshot(&after)) {
                if field == name {
                    assert_ne!(was, now, "set_{name} did not change {field}");
                } else {
                    assert_eq!(was, now, "set_{name} disturbed {field}");
                }
            }
        }
    }

    fn piano_snapshot(p: &PianoPanel) -> Vec<(&'static str, String)> {
        vec![
            ("category", format!("{}", p.category())),
            ("piano_model", format!("{}", p.piano_model())),
            ("clav_model", format!("{}", p.clav_model())),
            ("acoustics", format!("{}", p.acoustics())),
            ("touch", format!("{}", p.touch())),
            ("mono", format!("{}", p.mono())),
            ("id", format!("{}", p.id())),
        ]
    }

    #[test]
    fn every_piano_panel_setter_moves_only_its_own_field() {
        each_setter_moves_only_its_own_field(
            || {
                let mut p = PianoPanel::default();
                p.set_category(5).unwrap();
                p.set_piano_model(17).unwrap();
                p.set_clav_model(2).unwrap();
                p.set_acoustics(1).unwrap();
                p.set_touch(2).unwrap();
                p.set_mono(true);
                p.set_id(0x6dd3_4782);
                p
            },
            piano_snapshot,
            vec![
                ("category", |p| p.set_category(2).unwrap()),
                ("piano_model", |p| p.set_piano_model(4).unwrap()),
                ("clav_model", |p| p.set_clav_model(1).unwrap()),
                ("acoustics", |p| p.set_acoustics(3).unwrap()),
                ("touch", |p| p.set_touch(0).unwrap()),
                ("mono", |p| p.set_mono(false)),
                ("id", |p| p.set_id(0xffff_ffff)),
            ],
        );
    }

    /// The gap bits `PianoPanel` is the RFC's example of: 60..59 between `category` and
    /// `piano_model`, and 53..49 above `clav_model`. B+ needs no declaration for them,
    /// and no setter may disturb them.
    #[test]
    fn the_piano_panels_unnamed_bits_survive_every_setter() {
        const GAPS: u64 = (0b11 << 59) | (0b11111 << 49) | 0b11_1111_1111;

        let mutations: Vec<fn(&mut PianoPanel)> = vec![
            |p| p.set_category(7).unwrap(),
            |p| p.set_piano_model(31).unwrap(),
            |p| p.set_clav_model(3).unwrap(),
            |p| p.set_acoustics(3).unwrap(),
            |p| p.set_touch(3).unwrap(),
            |p| p.set_mono(true),
            |p| p.set_id(0xffff_ffff),
        ];

        for mutate in mutations {
            let mut panel = PianoPanel { settings: u64::MAX };
            mutate(&mut panel);
            assert_eq!(panel.settings & GAPS, GAPS, "a setter cleared a gap bit");

            let mut panel = PianoPanel { settings: 0 };
            mutate(&mut panel);
            assert_eq!(panel.settings & GAPS, 0, "a setter set a gap bit");
        }
    }

    fn sample_snapshot(p: &SamplePanel) -> Vec<(&'static str, String)> {
        vec![
            ("attack", format!("{}", p.attack())),
            ("decay_release", format!("{}", p.decay_release())),
            ("number", format!("{}", p.number())),
            ("id", format!("{}", p.id())),
            ("dynamics", format!("{}", p.dynamics())),
            ("filter", format!("{}", p.filter())),
        ]
    }

    #[test]
    fn every_sample_panel_setter_moves_only_its_own_field() {
        each_setter_moves_only_its_own_field(
            || {
                let mut p = SamplePanel::default();
                p.set_attack(96).unwrap();
                p.set_decay_release(64).unwrap();
                p.set_number(158);
                p.set_id(0x89be_e289);
                p.set_dynamics(2).unwrap();
                p.set_filter(true);
                p
            },
            sample_snapshot,
            vec![
                ("attack", |p| p.set_attack(0).unwrap()),
                ("decay_release", |p| p.set_decay_release(127).unwrap()),
                ("number", |p| p.set_number(0)),
                ("id", |p| p.set_id(0)),
                ("dynamics", |p| p.set_dynamics(1).unwrap()),
                ("filter", |p| p.set_filter(false)),
            ],
        );
    }

    fn effects_snapshot(p: &EffectsPanel) -> Vec<(&'static str, String)> {
        vec![
            ("fx1", format!("{}", p.fx1())),
            ("fx1_type", format!("{}", p.fx1_type())),
            ("fx1_rate", format!("{}", p.fx1_rate())),
            ("fx2", format!("{}", p.fx2())),
            ("fx2_type", format!("{}", p.fx2_type())),
            ("fx2_rate", format!("{}", p.fx2_rate())),
            ("fx3", format!("{}", p.fx3())),
            ("fx3_type", format!("{}", p.fx3_type())),
            ("fx3_compression", format!("{}", p.fx3_compression())),
            ("fx4", format!("{}", p.fx4())),
            ("fx4_feedback", format!("{}", p.fx4_feedback())),
            ("fx4_tempo", format!("{}", p.fx4_tempo())),
            ("fx4_moisture", format!("{}", p.fx4_moisture())),
            ("fx4_ping_pong", format!("{}", p.fx4_ping_pong())),
            ("fx5", format!("{}", p.fx5())),
            ("fx5_type", format!("{}", p.fx5_type())),
            ("fx5_moisture", format!("{}", p.fx5_moisture())),
            ("equalizer_on", format!("{}", p.equalizer_on())),
            ("equalizer_freq", format!("{}", p.equalizer_freq())),
            (
                "equalizer_freq_gain",
                format!("{}", p.equalizer_freq_gain()),
            ),
            ("equalizer_bass", format!("{}", p.equalizer_bass())),
            ("equalizer_treble", format!("{}", p.equalizer_treble())),
            ("rotary_stop", format!("{}", p.rotary_stop())),
            ("rotary_speed", format!("{}", p.rotary_speed())),
        ]
    }

    fn busy_effects() -> EffectsPanel {
        let mut p = EffectsPanel::default();
        p.set_fx1(3).unwrap();
        p.set_fx1_type(9).unwrap();
        p.set_fx1_rate(101).unwrap();
        p.set_fx2(2).unwrap();
        p.set_fx2_type(5).unwrap();
        p.set_fx2_rate(37).unwrap();
        p.set_fx3(3).unwrap();
        p.set_fx3_type(6).unwrap();
        p.set_fx3_compression(120).unwrap();
        p.set_fx4(2).unwrap();
        p.set_fx4_feedback(3).unwrap();
        p.set_fx4_tempo(64).unwrap();
        p.set_fx4_moisture(90).unwrap();
        p.set_fx4_ping_pong(true);
        p.set_fx5(true);
        p.set_fx5_type(5).unwrap();
        p.set_fx5_moisture(0x55).unwrap();
        p.set_equalizer_on(true);
        p.set_equalizer_freq(70).unwrap();
        p.set_equalizer_freq_gain(0x2a).unwrap();
        p.set_equalizer_bass(33).unwrap();
        p.set_equalizer_treble(99).unwrap();
        p.set_rotary_stop(1).unwrap();
        p.set_rotary_speed(1).unwrap();
        p
    }

    /// The panel that holds both cross-word fields. `equalizer_freq_gain` and
    /// `fx5_moisture` are the ones worth watching: each writes into two backing words,
    /// so a wrong half would show up as a neighbour moving.
    #[test]
    fn every_effects_panel_setter_moves_only_its_own_field() {
        each_setter_moves_only_its_own_field(
            busy_effects,
            effects_snapshot,
            vec![
                ("fx1", |p| p.set_fx1(0).unwrap()),
                ("fx1_type", |p| p.set_fx1_type(2).unwrap()),
                ("fx1_rate", |p| p.set_fx1_rate(0).unwrap()),
                ("fx2", |p| p.set_fx2(1).unwrap()),
                ("fx2_type", |p| p.set_fx2_type(0).unwrap()),
                ("fx2_rate", |p| p.set_fx2_rate(127).unwrap()),
                ("fx3", |p| p.set_fx3(1).unwrap()),
                ("fx3_type", |p| p.set_fx3_type(0).unwrap()),
                ("fx3_compression", |p| p.set_fx3_compression(0).unwrap()),
                ("fx4", |p| p.set_fx4(0).unwrap()),
                ("fx4_feedback", |p| p.set_fx4_feedback(0).unwrap()),
                ("fx4_tempo", |p| p.set_fx4_tempo(127).unwrap()),
                ("fx4_moisture", |p| p.set_fx4_moisture(0).unwrap()),
                ("fx4_ping_pong", |p| p.set_fx4_ping_pong(false)),
                ("fx5", |p| p.set_fx5(false)),
                ("fx5_type", |p| p.set_fx5_type(0).unwrap()),
                ("fx5_moisture", |p| p.set_fx5_moisture(0x2a).unwrap()),
                ("equalizer_on", |p| p.set_equalizer_on(false)),
                ("equalizer_freq", |p| p.set_equalizer_freq(0).unwrap()),
                ("equalizer_freq_gain", |p| {
                    p.set_equalizer_freq_gain(0x55).unwrap()
                }),
                ("equalizer_bass", |p| p.set_equalizer_bass(127).unwrap()),
                ("equalizer_treble", |p| p.set_equalizer_treble(0).unwrap()),
                ("rotary_stop", |p| p.set_rotary_stop(0).unwrap()),
                ("rotary_speed", |p| p.set_rotary_speed(0).unwrap()),
            ],
        );
    }

    /// A cross-word write must land in both words and disturb neither neighbour. The
    /// halves are unequal — three bits high and four low for the EQ gain, five and two
    /// for the delay mix — so getting the split backwards is a live failure mode.
    #[test]
    fn a_cross_word_field_writes_both_halves() {
        let mut p = busy_effects();

        // 0b1010101: high 0b101 into settings 2..0, low 0b0101 into settings2 31..28.
        p.set_equalizer_freq_gain(0b1010101).unwrap();
        assert_eq!(p.settings & 0b111, 0b101);
        assert_eq!(p.settings2 >> 28, 0b0101);
        assert_eq!(p.equalizer_freq_gain(), 0b1010101);

        // 0b1010101: high 0b10101 into settings2 4..0, low 0b01 into settings3 7..6.
        p.set_fx5_moisture(0b1010101).unwrap();
        assert_eq!(p.settings2 & 0b11111, 0b10101);
        assert_eq!(p.settings3 >> 6, 0b01);
        assert_eq!(p.fx5_moisture(), 0b1010101);

        // Both are seven bits wide, spread over two words; 0x80 fits in neither.
        assert_eq!(p.set_equalizer_freq_gain(0x80).unwrap_err().width, 7);
        assert_eq!(p.set_fx5_moisture(0x80).unwrap_err().width, 7);
    }

    fn extra_snapshot(p: &Extra) -> Vec<(&'static str, String)> {
        vec![
            ("fx1_control", format!("{}", p.fx1_control())),
            ("fx2_deep", format!("{}", p.fx2_deep())),
            ("equalizer_part", format!("{}", p.equalizer_part())),
        ]
    }

    #[test]
    fn every_extra_setter_moves_only_its_own_field() {
        each_setter_moves_only_its_own_field(
            || {
                let mut p = Extra::default();
                p.set_fx1_control(true);
                p.set_fx2_deep(true);
                p.set_equalizer_part(2).unwrap();
                p
            },
            extra_snapshot,
            vec![
                ("fx1_control", |p| p.set_fx1_control(false)),
                ("fx2_deep", |p| p.set_fx2_deep(false)),
                ("equalizer_part", |p| p.set_equalizer_part(1).unwrap()),
            ],
        );
    }

    /// The organ panel had no write path at all. These check the round trip through its
    /// existing byte-array shape, including the two places the format overloads a byte.
    #[test]
    fn organ_writes_round_trip_through_their_accessors() {
        use OrganModel::*;

        let mut organ = OrganPanel::default();

        for model in [B3, Vox, Farfisa, Pipe] {
            for preset in [1u8, 2] {
                organ.set_preset(model, preset);
                assert_eq!(organ.preset(model), preset);

                let bars = [8, 7, 6, 5, 4, 3, 2, 1, 0];
                organ.set_drawbars(model, preset, bars).unwrap();
                assert_eq!(organ.drawbars(model, preset), bars);

                organ.set_vib_on(model, preset, true);
                assert_eq!(organ.vib_on(model, preset), model != Pipe);
            }
        }

        // Presets do not alias: writing one must not move the other.
        organ.set_drawbars(Vox, 1, [1; 9]).unwrap();
        organ.set_drawbars(Vox, 2, [2; 9]).unwrap();
        assert_eq!(organ.drawbars(Vox, 1), [1; 9]);
        assert_eq!(organ.drawbars(Vox, 2), [2; 9]);

        // A drawbar is a physical position 0..=8; two share a byte, so an over-wide
        // value has to be refused rather than spill into its neighbour.
        assert!(organ
            .set_drawbars(B3, 1, [9, 0, 0, 0, 0, 0, 0, 0, 0])
            .is_err());
        assert_eq!(
            organ.drawbars(Vox, 1),
            [1; 9],
            "a refused write still wrote"
        );

        // Farfisa tabs are the >= 5 display transform, written back as 8 / 0.
        let tabs = [true, false, true, false, true, false, true, false, true];
        organ.set_farfisa_tabs(2, tabs);
        assert_eq!(organ.farfisa_tabs(2), tabs);
    }

    /// `0x51` carries the vib mode, the percussion third and the percussion speed, and
    /// `0x59` carries vibrato-on, percussion-on and the b3-bass drawbars. Neither may
    /// leak into the other — the same overlap the read-side tests above guard.
    #[test]
    fn organ_writes_do_not_leak_across_the_bytes_they_share() {
        use OrganModel::*;

        let mut organ = OrganPanel::default();
        organ.set_vib_type(B3, VibChorus::C3).unwrap();
        organ.set_b3_perc_third(true);
        organ.set_b3_perc_speed(PercSpeed::Soft);
        organ.set_b3_perc_on(1, true);
        organ.set_vib_on(B3, 1, true);
        organ.set_b3_bass_drawbars([8, 7]).unwrap();

        assert_eq!(organ.vib_type(B3), Some(VibChorus::C3));
        assert!(organ.b3_perc_third());
        assert_eq!(organ.b3_perc_speed(), PercSpeed::Soft);
        assert!(organ.b3_perc_on(1));
        assert!(organ.vib_on(B3, 1));
        assert_eq!(organ.b3_bass_drawbars(), [8, 7]);

        // Rewriting the bass bars must not disturb the flags sharing 0x59, and bar 9 of
        // the main block lives in that byte's high nibble.
        organ
            .set_drawbars(B3, 1, [0, 0, 0, 0, 0, 0, 0, 0, 6])
            .unwrap();
        organ.set_b3_bass_drawbars([3, 4]).unwrap();
        assert_eq!(organ.b3_bass_drawbars(), [3, 4]);
        assert!(organ.b3_perc_on(1), "the bass write cleared percussion-on");
        assert!(organ.vib_on(B3, 1), "the bass write cleared vibrato-on");
        assert_eq!(organ.drawbars(B3, 1)[8], 6, "the bass write moved bar 9");
    }

    /// A model only offers a subset of the six vib/chorus modes, and Pipe offers none.
    /// The stored value is an index into that subset, so a mode the model does not have
    /// cannot be encoded at all — an error, not a silent nearest match.
    #[test]
    fn an_unavailable_vibrato_mode_is_refused() {
        use OrganModel::*;

        let mut organ = OrganPanel::default();
        assert!(organ.set_vib_type(Vox, VibChorus::V3).is_ok());
        assert!(organ.set_vib_type(Vox, VibChorus::C1).is_err());
        assert!(organ.set_vib_type(Farfisa, VibChorus::C2).is_ok());
        assert!(organ.set_vib_type(Pipe, VibChorus::V1).is_err());
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
