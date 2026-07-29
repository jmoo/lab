use crate::bits::{Field, FieldOverflow};
use crate::common;
use crate::common::{bank, PartMix};
use crate::crc::{CrcReader, CrcWriter};
use crate::electro5::{Instrument, OctaveShift, SplitPoint, Transpose};
use crate::error::ParseError;
use crate::types::RangedU16Pair;
use binrw::{binrw, BinRead, BinReaderExt, BinWriterExt};
use nord_bits_derive::bitpanel;

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

// 0x2e-0x34 — the centre panel.
//
// **G+**: the panel is an ordinary Rust struct with `pub` fields, and each field carries
// the bits it occupies. `#[bitpanel]` derives the packed `CenterPanelWords`, both
// directions of the conversion, and a `Debug` over the decoded values. Nothing about the
// position is spelled twice, and nothing about the encoding is written by hand.
//
// The field's *type* decides how it is written: a raw `u8` may be wider than its slot, so
// it is encoded with a checked write; a domain type carries its range, so it is encoded
// with the unchecked one and `Field` proves the fit at compile time. See the
// `nord-bits-derive` module docs.
//
// Bits no field claims — `settings3` 7..0, and the rest — survive in the private word
// copy, so a panel still migrates one field at a time.

#[bitpanel(settings: u16, settings2: u8, settings3: u32)]
#[derive(Default)]
pub struct CenterPanel {
    #[bits(settings, 15..=13)]
    pub left_part: Instrument,
    #[bits(settings, 12..=10)]
    pub right_part: Instrument,
    #[bits(settings, 9..=6)]
    pub left_octave_shift: OctaveShift,
    #[bits(settings, 5..=2)]
    pub right_octave_shift: OctaveShift,
    #[bits(settings, 1..=1)]
    pub left_sustain: bool,
    #[bits(settings, 0..=0)]
    pub right_sustain: bool,

    #[bits(settings2, 7..=7)]
    pub left_control: bool,
    #[bits(settings2, 6..=6)]
    pub right_control: bool,
    /// Always zero in every corpus specimen. Named so it is visible, not so it is used.
    #[bits(settings2, 5..=5)]
    pub unknown_boolean1: bool,
    #[bits(settings2, 4..=4)]
    pub split: bool,
    #[bits(settings2, 3..=1)]
    pub split_point: SplitPoint,
    /// NOTE: the Electro 5 sometimes leaves this true even when the transpose is 0. It
    /// shows no transpose light when that happens.
    #[bits(settings2, 0..=0)]
    pub transpose_enabled: bool,

    /// Half-step transposition, `-6..=6`, stored biased by 6.
    #[bits(settings3, 31..=28)]
    pub transpose: Transpose,
    #[bits(settings3, 27..=21)]
    pub part_mix: PartMix,
    /// 0..=127, shown on the panel as 0..10.
    #[bits(settings3, 20..=14)]
    pub gain: u8,
    /// `0` b3, `1` b3+bass, `2` pipe, `3` vox, `4` farfisa.
    #[bits(settings3, 13..=11)]
    pub organ_type: u8,
    #[bits(settings3, 10..=10)]
    pub lower_enabled: bool,
    #[bits(settings3, 9..=9)]
    pub upper_enabled: bool,
    #[bits(settings3, 8..=8)]
    pub drawbar_live: bool,
}

// 0x3a..0x41 — the piano panel. Bits 60..59 and 53..49 are named by nothing.

#[bitpanel(settings: u64)]
#[derive(Default)]
pub struct PianoPanel {
    /// 5 == 0, 6 == 1, 1 == 2, 2 == 3, 3 == 4, 4 == 5.
    #[bits(settings, 63..=61)]
    pub category: u8,
    /// Zero-based model slot *within* [`category`](Self::category) — the panel's
    /// Model dial. A slot coordinate, not an identity; see [`id`](Self::id).
    #[bits(settings, 58..=54)]
    pub piano_model: u8,
    #[bits(settings, 48..=47)]
    pub clav_model: u8,
    #[bits(settings, 46..=45)]
    pub acoustics: u8,
    #[bits(settings, 44..=43)]
    pub touch: u8,
    #[bits(settings, 42..=42)]
    pub mono: bool,
    /// The piano (`.npno`) this program depends on: a stable 32-bit id in bits
    /// 41..=10, independent of where the piano sits in the instrument's library.
    /// `0` means "no piano referenced". This — not
    /// [`category`](Self::category)/[`piano_model`](Self::piano_model), which are
    /// slot coordinates — is what resolves the song → program → piano chain, and
    /// what Nord Sound Manager checks to decide whether a Restore is missing a
    /// dependency.
    #[bits(settings, 41..=10)]
    pub id: u32,
}

// 0x46..0x4d — the sample panel.

#[bitpanel(settings: u64)]
#[derive(Default)]
pub struct SamplePanel {
    #[bits(settings, 63..=57)]
    pub attack: u8,
    #[bits(settings, 56..=50)]
    pub decay_release: u8,
    /// Zero-based slot of the sample in the instrument's Samp Lib, i.e. the
    /// number shown on the panel minus one. This is a *position*, not an
    /// identity: adding or deleting samples renumbers it, and the corpus has
    /// ids that appear under several numbers (and numbers reused by several
    /// ids). Use [`id`](Self::id) to resolve the dependency.
    #[bits(settings, 49..=42)]
    pub number: u8,
    /// The sample (`.nsmp`) this program depends on: a stable 32-bit id in bits
    /// 41..=10, laid out exactly as [`PianoPanel::id`]. Same range, same type — the
    /// first shared component, reused across two panels before a second device model
    /// exists to reuse it across.
    #[bits(settings, 41..=10)]
    pub id: u32,
    #[bits(settings, 9..=8)]
    pub dynamics: u8,
    #[bits(settings, 7..=7)]
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

/// `[u8; 69]` has no `Default` — the std impls stop at 32 — so this one is written out
/// rather than derived like the other panels'.
impl Default for OrganPanel {
    fn default() -> Self {
        OrganPanel {
            raw: [0; ORGAN_LEN],
        }
    }
}

// 0x93..0x9f — the effects panel, which holds the format's two cross-word fields. Under
// `#[bitpanel]` a straddle is just a placement with two halves; nothing at the call site,
// in the struct, or in the conversion treats it specially.

#[bitpanel(settings: u64, settings2: u32, settings3: u8)]
#[derive(Default)]
pub struct EffectsPanel {
    /// 0: off, 1: lower, 2: upper.
    #[bits(settings, 63..=62)]
    pub fx1: u8,
    /// 1: pan1, pan2, pan1&2; 2: wah, rm, trem1, trem2, trem1&2.
    #[bits(settings, 61..=58)]
    pub fx1_type: u8,
    /// 0..127, shown as 0..10.
    #[bits(settings, 57..=51)]
    pub fx1_rate: u8,
    /// 0: off, 1: lower, 2: upper.
    #[bits(settings, 50..=49)]
    pub fx2: u8,
    /// flang, choir1, choir2, vibe, phas1, phas2.
    #[bits(settings, 48..=45)]
    pub fx2_type: u8,
    /// 0..127, shown as 0..10.
    #[bits(settings, 44..=38)]
    pub fx2_rate: u8,
    /// 0: off, 1: lower, 2: upper.
    #[bits(settings, 37..=36)]
    pub fx4: u8,
    #[bits(settings, 35..=34)]
    pub fx4_feedback: u8,
    /// 0..127, 750ms..20ms.
    #[bits(settings, 33..=27)]
    pub fx4_tempo: u8,
    /// Delay wet/dry, 0..127, shown as 0..10.
    #[bits(settings, 26..=20)]
    pub fx4_moisture: u8,
    #[bits(settings, 19..=19)]
    pub fx4_ping_pong: bool,
    /// EQ engaged. **Which part it applies to is not in this word** — see
    /// [`Extra::equalizer_part`].
    ///
    /// This was previously decoded as a two-bit `equalizer_part_select`, which could
    /// only ever answer 0 or 2: diffing the four named `equalizer/{0,1,2,3}_…`
    /// specimens shows they are byte-identical across `0x93..0x9a` apart from this
    /// single bit, and the lower/upper/both choice lives at `0xa1`.
    #[bits(settings, 18..=18)]
    pub equalizer_on: bool,
    #[bits(settings, 16..=10)]
    pub equalizer_freq: u8,
    #[bits(settings, 9..=3)]
    pub equalizer_treble: u8,

    /// Split across two words: three bits at the bottom of 0x9a, four at the top of 0x9b.
    #[bits(settings, 2..=0, settings2, 31..=28)]
    pub equalizer_freq_gain: u8,

    #[bits(settings2, 27..=21)]
    pub equalizer_bass: u8,
    /// 0: off, 1: lower, 2: upper.
    #[bits(settings2, 20..=19)]
    pub fx3: u8,
    /// none, twin, rotary, comp, small, jc.
    #[bits(settings2, 18..=16)]
    pub fx3_type: u8,
    /// 0..127, shown as 0..10.
    #[bits(settings2, 15..=9)]
    pub fx3_compression: u8,
    #[bits(settings2, 8..=8)]
    pub fx5: bool,
    #[bits(settings2, 7..=5)]
    pub fx5_type: u8,

    /// The other one: five bits at the bottom of 0x9e, two at the top of 0x9f.
    #[bits(settings2, 4..=0, settings3, 7..=6)]
    pub fx5_moisture: u8,

    /// 0 = off, 1 = on.
    #[bits(settings3, 5..=5)]
    pub rotary_stop: u8,
    /// 0 = slow, 1 = fast.
    #[bits(settings3, 4..=4)]
    pub rotary_speed: u8,
}

// 0xa1..0xa4

#[bitpanel(settings: u32)]
#[derive(Default)]
pub struct Extra {
    /// fx1 control pedal.
    #[bits(settings, 28..=28)]
    pub fx1_control: bool,
    /// fx2 deep.
    #[bits(settings, 27..=27)]
    pub fx2_deep: bool,
    /// Which part the equalizer applies to: `0` lower, `1` upper, `2` lower+upper.
    ///
    /// Whether the EQ is engaged at all is a separate bit,
    /// [`EffectsPanel::equalizer_on`] — so `0` here means *lower*, not *off*. Located
    /// by diffing the `equalizer/{0,1,2,3}_…` specimens, which differ only at `0xa1`
    /// (and in the enable bit and CRC).
    #[bits(settings, 26..=25)]
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
    //
    // `binrw` reads the packed words and `try_map` converts, so decoding happens inside
    // `BinRead` and a file with an impossible value fails to parse. This is discussion
    // #184's sanctioned pattern: the packing lives in a mapped companion, never in
    // `binrw` itself.
    #[br(try_map = |w: CenterPanelWords| CenterPanel::try_from(w))]
    #[bw(try_map = |p: &CenterPanel| CenterPanelWords::try_from(p))]
    pub center_panel: CenterPanel,

    // 0x35..0x3b
    pad1: [u8; (0x39 - 0x34) as usize],

    // 0x3a..0x41
    #[br(try_map = |w: PianoPanelWords| PianoPanel::try_from(w))]
    #[bw(try_map = |p: &PianoPanel| PianoPanelWords::try_from(p))]
    pub piano_panel: PianoPanel,

    // 0x42..0x45
    pad2: [u8; (0x45 - 0x41) as usize],

    // 0x46..0x4d
    #[br(try_map = |w: SamplePanelWords| SamplePanel::try_from(w))]
    #[bw(try_map = |p: &SamplePanel| SamplePanelWords::try_from(p))]
    pub sample_panel: SamplePanel,

    // 0x4e..0x92
    pub organ_panel: OrganPanel,

    // 0x93..0x9f
    #[br(try_map = |w: EffectsPanelWords| EffectsPanel::try_from(w))]
    #[bw(try_map = |p: &EffectsPanel| EffectsPanelWords::try_from(p))]
    pub effects_panel: EffectsPanel,

    // 0xa0
    todo: u8,

    // 0xa1..0xa4
    #[br(try_map = |w: ExtraWords| Extra::try_from(w))]
    #[bw(try_map = |p: &Extra| ExtraWords::try_from(p))]
    pub extra: Extra,
}

#[derive(Debug)]
pub struct Program {
    pub schema: Schema,
    location: Location,
    name: Option<String>,
}

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

    pub fn lower_part(&self) -> Instrument {
        self.schema.center_panel.left_part
    }

    pub fn upper_part(&self) -> Instrument {
        self.schema.center_panel.right_part
    }

    pub fn lower_octave_shift(&self) -> OctaveShift {
        self.schema.center_panel.left_octave_shift
    }

    pub fn upper_octave_shift(&self) -> OctaveShift {
        self.schema.center_panel.right_octave_shift
    }

    pub fn lower_sustain(&self) -> bool {
        self.schema.center_panel.left_sustain
    }

    pub fn upper_sustain(&self) -> bool {
        self.schema.center_panel.right_sustain
    }

    pub fn lower_control(&self) -> bool {
        self.schema.center_panel.left_control
    }

    pub fn upper_control(&self) -> bool {
        self.schema.center_panel.right_control
    }

    pub fn split_point(&self) -> SplitPoint {
        self.schema.center_panel.split_point
    }

    pub fn split(&self) -> bool {
        self.schema.center_panel.split
    }

    pub fn transpose(&self) -> Transpose {
        self.schema.center_panel.transpose
    }

    pub fn transpose_enabled(&self) -> bool {
        self.schema.center_panel.transpose_enabled
    }

    pub fn part_mix(&self) -> PartMix {
        self.schema.center_panel.part_mix
    }

    pub fn gain(&self) -> u8 {
        self.schema.center_panel.gain
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
        self.schema.center_panel.organ_type
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

    // ── the decoded-struct shape's costs and guarantees ─────────────────────────

    use std::io::Cursor;

    /// **The cost of `pub` fields.** A slot narrower than its Rust type can be assigned a
    /// value that does not fit, and nothing can object until encode time.
    ///
    /// `gain` is seven bits held as `u8`. `panel.gain = 200` is a legal assignment to a
    /// public field of a public struct: it compiles, it type-checks, and the panel now
    /// holds a value that cannot be written. Refusing at encode beats truncating — a
    /// truncation would silently corrupt `part_mix` next door — but the report arrives
    /// nowhere near the line that caused it.
    #[test]
    fn an_out_of_range_assignment_is_caught_only_at_write() {
        let mut program = Program::new((0, 0).try_into().unwrap());

        // Compiles. No error, no warning, no way to object.
        program.schema.center_panel.gain = 200;

        let mut bytes = Vec::new();
        let err = program
            .write_to(&mut Cursor::new(&mut bytes))
            .expect_err("200 does not fit seven bits");
        assert!(
            err.to_string().contains("does not fit"),
            "unhelpful error: {err}",
        );

        // And the failure lands mid-write: the stream already holds a partial program.
        // Callers going through `nord_format::to_bytes` are fine — it discards the
        // buffer — but a caller writing straight to a file is not.
        assert!(
            !bytes.is_empty() && bytes.len() < FILE_LEN,
            "expected a partial write, got {} of {FILE_LEN} bytes",
            bytes.len(),
        );
    }

    /// The same hazard exists on every raw-integer field in a narrow slot. Counted here
    /// so the comparison has a number rather than an impression.
    #[test]
    fn the_number_of_fields_that_can_be_assigned_an_unwritable_value() {
        // center: gain (7), organ_type (3)
        // piano:  category (3), piano_model (5), clav_model (2), acoustics (2), touch (2)
        // sample: attack (7), decay_release (7), dynamics (2)
        // fx:     fx1 (2), fx1_type (4), fx1_rate (7), fx2 (2), fx2_type (4), fx2_rate (7),
        //         fx3 (2), fx3_type (3), fx3_compression (7), fx4 (2), fx4_feedback (2),
        //         fx4_tempo (7), fx4_moisture (7), fx5_type (3), fx5_moisture (7),
        //         equalizer_freq (7), equalizer_freq_gain (7), equalizer_bass (7),
        //         equalizer_treble (7), rotary_stop (1), rotary_speed (1)
        // extra:  equalizer_part (2)
        //
        // Every one is a `pub u8` whose slot is narrower than eight bits.
        let narrow_pub_u8_fields = 2 + 5 + 3 + 21 + 1;
        assert_eq!(narrow_pub_u8_fields, 32);
    }

    /// Re-stamp the body CRC after corrupting a byte, so a decode test exercises the
    /// field check rather than the checksum.
    fn restamp_crc(bytes: &mut [u8]) {
        use crate::crc::MultipartCrc32;
        let mut crc = MultipartCrc32::new(0x2c, 0xa4 - 0x2c);
        crc.update(0, bytes);
        bytes[0x18..0x1c].copy_from_slice(&crc.checksum().to_le_bytes());
    }

    /// Validation is part of `BinRead`, not a step a caller has to remember.
    ///
    /// This shape gets it structurally: `#[br(try_map)]` runs the fallible decode inside
    /// the read, so `Schema::read_be` — public API that never touches
    /// [`Program::read_from`] — validates too, with nothing to forget. Note there is no
    /// way to build the corrupt input through the API at all: `left_part` is an
    /// `Instrument`, so a panel in memory *cannot* hold the invalid value. It has to be
    /// forged in the bytes.
    #[test]
    fn no_decode_path_can_skip_validation() {
        use binrw::BinRead;

        let mut program = Program::new((0, 0).try_into().unwrap());
        let mut bytes = Vec::new();
        program.write_to(&mut Cursor::new(&mut bytes)).unwrap();

        // Self-check: re-stamping an untouched file must be a no-op.
        let pristine = bytes.clone();
        restamp_crc(&mut bytes);
        assert_eq!(bytes, pristine, "the CRC helper does not match the writer");

        // 0b111 is not an `Instrument`.
        bytes[0x2e] |= 0b1110_0000;
        restamp_crc(&mut bytes);

        let front = Program::read_from(&mut Cursor::new(&mut bytes))
            .expect_err("the front door accepted an undecodable panel");
        assert!(
            front.to_string().contains("exceeds bound"),
            "refused for the wrong reason: {front}",
        );
        assert!(
            Schema::read_be(&mut Cursor::new(&bytes)).is_err(),
            "`Schema::read_be` accepted an undecodable panel",
        );
        assert!(
            CenterPanel::try_from(
                CenterPanelWords::read_be(&mut Cursor::new(&bytes[0x2e..0x35])).unwrap()
            )
            .is_err(),
            "the conversion itself accepted an undecodable panel",
        );
    }

    /// Bits no field names survive a re-encode, because each panel keeps the words it was
    /// decoded from. This is what the private `words` copy buys, and the reason the shape
    /// can migrate one field at a time.
    #[test]
    fn unnamed_bits_survive_a_re_encode() {
        // `PianoPanel` 60..59 and 53..49 are named by nothing.
        const GAPS: u64 = (0b11 << 59) | (0b11111 << 49);

        let mut words = PianoPanelWords { settings: 0 };
        words.settings |= GAPS;
        let mut panel = PianoPanel::try_from(words).unwrap();
        panel.category = 2;
        panel.id = 0xdead_beef;

        let out = PianoPanelWords::try_from(&panel).unwrap();
        assert_eq!(out.settings & GAPS, GAPS, "a re-encode cleared a gap bit");

        let mut panel = PianoPanel::try_from(PianoPanelWords { settings: 0 }).unwrap();
        panel.category = 7;
        panel.piano_model = 31;
        let out = PianoPanelWords::try_from(&panel).unwrap();
        assert_eq!(out.settings & GAPS, 0, "a re-encode set a gap bit");
    }

    /// Decode and encode are inverses on any word the decoder accepts.
    #[test]
    fn decode_and_encode_are_inverse() {
        for pattern in [0u64, u64::MAX, 0xa5a5_a5a5_a5a5_a5a5, 0x5a5a_5a5a_5a5a_5a5a] {
            let words = PianoPanelWords { settings: pattern };
            let panel = PianoPanel::try_from(words).unwrap();
            assert_eq!(PianoPanelWords::try_from(&panel).unwrap().settings, pattern);

            let words = SamplePanelWords { settings: pattern };
            let panel = SamplePanel::try_from(words).unwrap();
            assert_eq!(
                SamplePanelWords::try_from(&panel).unwrap().settings,
                pattern
            );

            let words = CenterPanelWords {
                settings: pattern as u16,
                settings2: pattern as u8,
                settings3: pattern as u32,
            };
            if let Ok(panel) = CenterPanel::try_from(words) {
                let out = CenterPanelWords::try_from(&panel).unwrap();
                assert_eq!(
                    (out.settings, out.settings2, out.settings3),
                    (words.settings, words.settings2, words.settings3),
                );
            }
        }
    }

    /// A default panel has to encode, which a zeroed word alone would not: an octave
    /// shift of zero is stored as 7, so all-zero bits decode as -7 — out of range. Here
    /// the derived `Default` gives the *fields* their proper defaults and the encode
    /// lays them over the zeroed words, so it comes out right without special handling.
    #[test]
    fn the_default_panel_encodes_and_decodes() {
        let panel = CenterPanel::default();
        let words = CenterPanelWords::try_from(&panel).unwrap();
        let back = CenterPanel::try_from(words).expect("default panel must decode");

        assert_eq!(back.left_octave_shift, 0);
        assert_eq!(back.right_octave_shift, 0);
        assert_eq!(back.transpose, 0);
        assert_eq!(back.left_part, Instrument::Organ);
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
}
