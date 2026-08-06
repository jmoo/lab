//! The Electro 5 program format (`.ne5p`).
//!
//! Reads top-down: the format's constants, then [`Schema`] — the body as `binrw` sees
//! it — then [`Program`], which is a `Schema` plus the container it came in. Each panel
//! is a `#[bitpanel]` in its own module.
//!
//! The live buffer ([`crate::electro5::live`]) is this same body under the tag `ne5l`,
//! addressed in three slots instead of eight banks of fifty. Only the container differs,
//! so both formats read one [`Schema`].

mod center;
mod effects;
mod organ;
mod piano;
mod sample;

pub use center::{CenterPanel, OrganType};
pub use effects::{EffectsPanel, EqualizerPart, Fx1Type, Fx2Type, Fx3Type, Fx5Type, Routing};
pub use organ::{B3PercSpeed, B3Vib, Drawbars, FarfisaVib, OrganModel, OrganPanel, VoxVib};
pub use piano::{PianoCategory, PianoPanel};
pub use sample::SamplePanel;

use crate::common::container::{self, Header};
use crate::error::Error;
use crate::file::{sealed, BodyReader, File, Format};
use crate::panel::{FieldError, Panel};
use crate::types::RangedU16Pair;
use binrw::{binrw, BinRead, BinWriterExt};

use std::io::{Cursor, Seek, Write};

pub const FORMAT: &str = "ne5p";
/// Schema versions this build's field offsets have been validated against. Every corpus
/// program reports 4. See [`crate::error::ParseError::UnsupportedVersion`].
pub const KNOWN_VERSIONS: &[u32] = &[4];
/// What a newly authored program is stamped with. Reading a file overwrites it with
/// whatever the file carried.
pub const DEFAULT_VERSION: u32 = 4;
/// The panel body, everything a `.ne5p` holds below its CBIN header.
pub const BODY_LEN: usize = 121;
/// Total file length: 44-byte CBIN header + the body.
pub const FILE_LEN: usize = container::HEADER_LEN + BODY_LEN;
pub const BANK_COUNT: u16 = 8;
pub const SLOT_COUNT: u16 = 50;

pub type Location = RangedU16Pair<BANK_COUNT, SLOT_COUNT>;

/// The 121-byte panel body.
///
/// The `ne5p` program and the `ne5l` live buffer disagree about nothing below the
/// header: the bodies are byte-identical, confirmed on hardware, and only the tag and
/// slot space differ — both of which live on the container.
///
/// ⚠️ The offsets below are absolute in a **type-1 file**, the way the panel modules and
/// the format notes write them; the body itself starts at [`container::HEADER_LEN`]. A
/// type-0 file holds the same body 20 bytes earlier.
#[binrw]
#[derive(Debug)]
#[brw(little)]
pub struct Schema {
    // 0x2c..0x2d
    #[brw(big)]
    program_version: u16,

    // 0x2e..0x34
    //
    // Decoding sits inside `try_map`, so a file with an impossible value fails to parse
    // rather than reaching a caller.
    #[br(try_map = |raw: [u8; 7]| CenterPanel::try_from(raw))]
    #[bw(map = |p: &CenterPanel| <[u8; 7]>::from(p))]
    pub center_panel: CenterPanel,

    // 0x35..0x3b
    pad1: [u8; (0x39 - 0x34) as usize],

    // 0x3a..0x41
    #[br(try_map = |raw: [u8; 8]| PianoPanel::try_from(raw))]
    #[bw(map = |p: &PianoPanel| <[u8; 8]>::from(p))]
    pub piano_panel: PianoPanel,

    // 0x42..0x45
    pad2: [u8; (0x45 - 0x41) as usize],

    // 0x46..0x4d
    #[br(try_map = |raw: [u8; 8]| SamplePanel::try_from(raw))]
    #[bw(map = |p: &SamplePanel| <[u8; 8]>::from(p))]
    pub sample_panel: SamplePanel,

    // 0x4e..0x92
    #[br(try_map = |raw: [u8; 69]| OrganPanel::try_from(raw))]
    #[bw(map = |p: &OrganPanel| <[u8; 69]>::from(p))]
    pub organ_panel: OrganPanel,

    // 0x93..0xa4
    #[br(try_map = |raw: [u8; 18]| EffectsPanel::try_from(raw))]
    #[bw(map = |p: &EffectsPanel| <[u8; 18]>::from(p))]
    pub effects_panel: EffectsPanel,
}

/// One settable field, addressed the way `--set` addresses it.
pub struct Field {
    /// `center_panel.transpose`.
    pub path: String,
    pub spec: crate::panel::FieldSpec,
    /// What the field currently holds, spelled the way [`Schema::set_field`] takes it.
    /// Feeding this straight back is always a no-op.
    pub value: String,
    /// The same value as `nord inspect` renders it. Differs from `value` only for a
    /// field too wide to have named values, where the rendering is a list and the
    /// spelling is the stored bits.
    pub display: String,
}

/// Describe one panel's fields under their qualified paths.
///
/// `field_specs` and `field_values` are emitted in declaration order and describe the
/// same fields, so the positional zip is sound — see `nord_bits_derive`.
pub(crate) fn describe<P: Panel>(prefix: &str, panel: &P) -> Vec<Field> {
    P::field_specs()
        .into_iter()
        .zip(panel.field_values())
        .map(|(spec, value)| Field {
            path: format!("{prefix}.{}", spec.name),
            value: crate::panel::settable_form(spec.width, &value.value, value.bits),
            display: value.value,
            spec,
        })
        .collect()
}

impl Schema {
    /// A body of default panels.
    pub fn new() -> Schema {
        Schema {
            pad1: [0; (0x39 - 0x34) as usize],
            pad2: [0; (0x45 - 0x41) as usize],
            program_version: 4,
            center_panel: CenterPanel::default(),
            piano_panel: PianoPanel::default(),
            sample_panel: SamplePanel::default(),
            organ_panel: OrganPanel::default(),
            effects_panel: EffectsPanel::default(),
        }
    }

    /// Every settable field of a program, in panel then declaration order.
    pub fn fields(&self) -> Vec<Field> {
        let mut out = describe("center_panel", &self.center_panel);
        out.extend(describe("piano_panel", &self.piano_panel));
        out.extend(describe("sample_panel", &self.sample_panel));
        out.extend(describe("organ_panel", &self.organ_panel));
        out.extend(describe("effects_panel", &self.effects_panel));
        out
    }

    /// Set one field, addressed as `panel.field`.
    ///
    /// ⚠️ The panel names are the one part of a path spelled by hand, so a panel added to
    /// the schema needs a line here and in [`Self::fields`]. Field names come from
    /// `#[bitpanel]` and cannot go stale.
    pub fn set_field(&mut self, path: &str, value: &str) -> Result<(), FieldError> {
        let (panel, field) = path
            .split_once('.')
            .ok_or_else(|| FieldError::UnknownField {
                panel: "a program",
                name: path.to_string(),
            })?;
        match panel {
            "center_panel" => self.center_panel.set_field(field, value),
            "piano_panel" => self.piano_panel.set_field(field, value),
            "sample_panel" => self.sample_panel.set_field(field, value),
            "organ_panel" => self.organ_panel.set_field(field, value),
            "effects_panel" => self.effects_panel.set_field(field, value),
            other => Err(FieldError::UnknownField {
                panel: "a program",
                name: other.to_string(),
            }),
        }
    }
}

impl Default for Schema {
    fn default() -> Schema {
        Schema::new()
    }
}

/// The `ne5p` format: one program — the [`Schema`] body in the eight-bank slot space.
#[derive(Debug)]
pub struct Ne5p;

impl sealed::Sealed for Ne5p {}

impl Format for Ne5p {
    const TAG: &'static str = FORMAT;
    const KNOWN_VERSIONS: &'static [u32] = KNOWN_VERSIONS;
    const FILE_LEN: Option<usize> = Some(FILE_LEN);
    type Location = Location;
    type Body = Schema;

    fn read_body(r: &mut BodyReader, _header: &Header) -> Result<Schema, Error> {
        Ok(Schema::read_be(&mut Cursor::new(r.bytes()?))?)
    }

    fn write_body(
        body: &Schema,
        _header: &Header,
        w: &mut (impl Write + Seek),
    ) -> Result<(), Error> {
        w.write_be(body)?;
        Ok(())
    }
}

pub type Program = File<Ne5p>;

impl File<Ne5p> {
    pub fn new(location: Location) -> Program {
        File {
            header: Header::new(FORMAT, DEFAULT_VERSION),
            location,
            body: Schema::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// An unknown schema version is refused at read, not decoded on a guess.
    ///
    /// Field offsets are only validated for the versions in the corpus. A future
    /// firmware bumping `ne5p` to 5 could move fields; decoding it with version-4
    /// offsets would yield plausible but wrong values, and writing it back would then
    /// persist them. Refusing is the only safe default.
    #[test]
    fn an_unknown_schema_version_is_refused() {
        use std::io::Cursor;

        let program = Program::new((0, 0).try_into().unwrap());
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
        // The refusal is a matchable variant carrying the facts, not a string.
        assert!(
            matches!(
                err,
                Error::Parse(crate::error::ParseError::UnsupportedVersion {
                    format: "ne5p",
                    version: 5,
                    ..
                })
            ),
            "unhelpful error: {err}",
        );
    }

    /// Every panel's encode is `From`, not `TryFrom`: no field can overrun its slot.
    ///
    /// The other half of that guarantee is not assertable from a test — giving a field a
    /// type wider than its slot is a const-eval panic out of `Field::FITS`, so retyping
    /// `PianoPanel::mono` from `bool` to `u8` fails to build rather than failing here.
    #[test]
    fn every_panels_encode_is_total() {
        fn total<P, W>(_: &P)
        where
            for<'a> W: From<&'a P>,
        {
        }

        let program = Program::new((0, 0).try_into().unwrap());
        total::<_, [u8; 7]>(&program.body.center_panel);
        total::<_, [u8; 8]>(&program.body.piano_panel);
        total::<_, [u8; 8]>(&program.body.sample_panel);
        total::<_, [u8; 18]>(&program.body.effects_panel);
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
    /// way to build the corrupt input through the API at all: `lower_part` is an
    /// `Instrument`, so a panel in memory *cannot* hold the invalid value. It has to be
    /// forged in the bytes.
    #[test]
    fn no_decode_path_can_skip_validation() {
        use binrw::BinRead;

        let program = Program::new((0, 0).try_into().unwrap());
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
        // Structural, not textual: the typed refusal must survive binrw's wrapping.
        assert!(
            matches!(
                front,
                Error::Parse(crate::error::ParseError::OutOfBounds { .. })
            ),
            "refused for the wrong reason: {front}",
        );
        assert!(
            Schema::read_be(&mut Cursor::new(&bytes[container::HEADER_LEN..])).is_err(),
            "`Schema::read_be` accepted an undecodable panel",
        );
        assert!(
            CenterPanel::try_from(<[u8; 7]>::try_from(&bytes[0x2e..0x35]).unwrap()).is_err(),
            "the conversion itself accepted an undecodable panel",
        );
    }

    /// A field set by name lands in the bits that field owns, and in no others.
    #[test]
    fn setting_a_field_by_name_moves_only_that_fields_bytes() {
        let mut program = Program::new((0, 0).try_into().unwrap());
        let mut before = Vec::new();
        program.write_to(&mut Cursor::new(&mut before)).unwrap();

        program
            .body
            .set_field("center_panel.transpose", "-5")
            .unwrap();
        assert_eq!(program.body.center_panel.transpose.inner(), -5);

        let mut after = Vec::new();
        program.write_to(&mut Cursor::new(&mut after)).unwrap();

        // `transpose` is bits 24..=27 of a panel starting at 0x2e, so byte 0x31 — plus
        // the body CRC at 0x18..0x1c, which every body change moves.
        let moved: Vec<usize> = (0..before.len())
            .filter(|&i| before[i] != after[i])
            .collect();
        assert_eq!(moved, vec![0x18, 0x19, 0x1a, 0x1b, 0x31], "{moved:x?}");
    }

    /// The library's field names are the CLI's arguments, so a path that does not exist
    /// has to say which half was wrong.
    #[test]
    fn an_unknown_path_names_what_it_could_not_find() {
        let mut program = Program::new((0, 0).try_into().unwrap());
        for (path, wanted) in [
            ("center_panel.nonesuch", "nonesuch"),
            ("nonesuch.transpose", "nonesuch"),
            ("transpose", "transpose"),
        ] {
            let err = program.body.set_field(path, "0").unwrap_err().to_string();
            assert!(err.contains(wanted), "{path}: {err}");
        }
    }

    /// Every field the panels declare is listed, and each lists a way to spell itself.
    #[test]
    fn every_declared_field_is_settable_by_its_listed_name() {
        let mut program = Program::new((0, 0).try_into().unwrap());
        let fields = program.body.fields();
        assert!(
            fields.iter().any(|f| f.path == "center_panel.transpose"),
            "the worked example is missing from the registry",
        );

        // Round-tripping every field through its own listed spelling is the property
        // that makes the registry usable: what `--fields` prints is what `--set` takes.
        for f in fields {
            let (path, value) = (f.path.clone(), f.value.clone());
            program
                .body
                .set_field(&path, &value)
                .unwrap_or_else(|e| panic!("{path} = {value:?}: {e}"));
        }
    }

    /// A nine-nibble drawbar block has no named values, so it is spelled by its bits —
    /// which for this field is also how a reader wants to see it.
    #[test]
    fn a_wide_field_is_spelled_by_its_stored_bits() {
        let mut program = Program::new((0, 0).try_into().unwrap());
        program
            .body
            .set_field("organ_panel.b3_preset1_drawbars", "0x087654321")
            .unwrap();
        assert_eq!(
            program.body.organ_panel.drawbars(OrganModel::B3, 1),
            [0, 8, 7, 6, 5, 4, 3, 2, 1],
        );

        let listed = program
            .body
            .fields()
            .into_iter()
            .find(|f| f.path == "organ_panel.b3_preset1_drawbars")
            .expect("declared");
        assert_eq!(listed.value, "0x87654321");
        assert_eq!(listed.display, "[0, 8, 7, 6, 5, 4, 3, 2, 1]");
    }

    /// Decode and encode are inverses on any bytes the decoder accepts.
    #[test]
    fn decode_and_encode_are_inverse() {
        for pattern in [0u64, u64::MAX, 0xa5a5_a5a5_a5a5_a5a5, 0x5a5a_5a5a_5a5a_5a5a] {
            let raw = pattern.to_be_bytes();
            let panel = PianoPanel::try_from(raw).unwrap();
            assert_eq!(<[u8; 8]>::from(&panel), raw);

            let panel = SamplePanel::try_from(raw).unwrap();
            assert_eq!(<[u8; 8]>::from(&panel), raw);

            let raw: [u8; 7] = raw[..7].try_into().unwrap();
            if let Ok(panel) = CenterPanel::try_from(raw) {
                assert_eq!(<[u8; 7]>::from(&panel), raw);
            }
        }
    }
}
