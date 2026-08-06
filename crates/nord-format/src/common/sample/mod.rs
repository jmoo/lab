//! Sample instruments (`.nsmp`) — the Nord Sample Library format.
//!
//! Shared across the Nord line rather than specific to one model, which is why this sits
//! in [`crate::common`]. The body inside the usual CBIN container is a chain of tagged
//! [`section`]s: an `hdr` carrying the name, a `cat` of category strings, a `map` ending
//! in the [`zone`] table, one [`stroke`] per zone, and a trailing `sty`.
//!
//! **The audio is encoded and stays that way.** Strokes are kept verbatim, so this reads
//! and rewrites instruments byte-exactly and can retune, rename and remap them — but it
//! cannot decode or synthesise the audio itself.

pub mod section;
pub mod stroke;
pub mod zone;

pub use section::Section;
pub use stroke::Stroke;
pub use zone::Zone;

use crate::common::container::Header;
use crate::error::{Error, ParseError};
use crate::file::{sealed, BodyReader, File, Format, Verbatim};
use std::fmt;
use std::io::{Cursor, Seek, Write};

pub const FORMAT: &str = "nsmp";

/// Offset of the instrument name within the `hdr` payload.
const NAME_AT: usize = 12;

/// Longest name this writer will emit.
///
/// The field is fixed-width and NUL-padded — a 4-character and a 14-character name give
/// the same file length — but only 14 bytes have ever been observed in use, and what
/// follows the name inside `hdr` is unmapped. Writing a longer one risks overwriting a
/// field we cannot see, so refuse instead. Reading is unrestricted.
pub const MAX_NAME_LEN: usize = 14;

/// A sample instrument's body: its sections in file order, including repeats — `stk`
/// appears once per zone.
#[derive(Clone, PartialEq, Eq)]
pub struct Sections(pub Vec<Section>);

impl fmt::Debug for Sections {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}

/// The `nsmp` format: a sample instrument.
///
/// A library file, not a slot save, so nothing about its header is gated:
///
/// * Its `version` is content as `format * 100 + revision` — a 2.0 instrument reads
///   200 — not a schema revision, since it moves with the library's own content.
/// * ⚠️ Its `trailer` is `0x000f0000` on every specimen, where every slotted format
///   holds the slot trailer. Meaning unknown; preserved verbatim.
/// * Its location is `0xFFFFFFFF` on every specimen — a library sample has no slot
///   until an instrument gives it one.
#[derive(Debug)]
pub struct Nsmp;

impl sealed::Sealed for Nsmp {}

impl Format for Nsmp {
    const TAG: &'static str = FORMAT;
    const KNOWN_VERSIONS: &'static [u32] = &[];
    const FILE_LEN: Option<usize> = None;
    type Location = Verbatim;
    type Body = Sections;

    fn read_body(r: &mut BodyReader, _header: &Header) -> Result<Sections, Error> {
        Ok(Sections(section::read_chain(&r.bytes()?, 0)?))
    }

    fn write_body(
        body: &Sections,
        _header: &Header,
        w: &mut (impl Write + Seek),
    ) -> Result<(), Error> {
        let mut out = Vec::new();
        for s in &body.0 {
            s.write_into(&mut out);
        }
        w.write_all(&out)?;
        Ok(())
    }

    fn check(_header: &Header) -> Result<(), ParseError> {
        Ok(())
    }
}

pub type Sample = File<Nsmp>;

impl File<Nsmp> {
    /// Decodes an instrument of either CBIN generation.
    pub fn from_bytes(bytes: &[u8]) -> Result<Sample, Error> {
        Sample::read_from(&mut Cursor::new(bytes))
    }

    /// Instrument name, as the Nord display shows it.
    ///
    /// The editor composes this from separate Main, Sub and Aux fields joined with `_`,
    /// so an empty Sub shows up as a doubled underscore rather than a typo.
    pub fn name(&self) -> Result<String, Error> {
        let hdr = self.hdr()?;
        let from = hdr.payload.get(NAME_AT..).ok_or_else(|| {
            ParseError::AssertFail(format!("hdr section is {} bytes", hdr.payload.len()))
        })?;
        let end = from.iter().position(|&b| b == 0).unwrap_or(from.len());
        Ok(String::from_utf8_lossy(&from[..end]).into_owned())
    }

    /// Renames in place, NUL-padding the rest of the field.
    pub fn set_name(&mut self, name: &str) -> Result<(), Error> {
        if name.len() > MAX_NAME_LEN {
            return Err(ParseError::OutOfBounds {
                value: format!("{name:?} ({} bytes)", name.len()),
                bound: format!("a name of at most {MAX_NAME_LEN} bytes"),
            }
            .into());
        }
        let hdr = section::find_mut(&mut self.body.0, section::HDR)
            .ok_or_else(|| ParseError::AssertFail("no hdr section".into()))?;
        let field = hdr
            .payload
            .get_mut(NAME_AT..NAME_AT + MAX_NAME_LEN)
            .ok_or_else(|| ParseError::AssertFail("hdr section is too short for a name".into()))?;
        field.fill(0);
        field[..name.len()].copy_from_slice(name.as_bytes());
        Ok(())
    }

    /// Keyboard zones, high to low.
    pub fn zones(&self) -> Result<Vec<Zone>, Error> {
        Ok(zone::read(&self.map()?.payload)?)
    }

    /// Sets one zone's top note. The strokes are untouched.
    pub fn set_zone_top_note(&mut self, index: usize, note: u8) -> Result<(), Error> {
        let map = section::find_mut(&mut self.body.0, section::MAP)
            .ok_or_else(|| ParseError::AssertFail("no map section".into()))?;
        zone::set_top_note(&mut map.payload, index, note)?;
        Ok(())
    }

    /// One stroke per zone, in the same high-to-low order as [`File::zones`].
    pub fn strokes(&self) -> Result<Vec<Stroke>, Error> {
        let zones = zone::count(&self.map()?.payload)?;
        self.stroke_sections()
            .enumerate()
            .map(|(i, s)| Ok(stroke::read(&s.payload, i, zones)?))
            .collect()
    }

    /// Retunes one zone by moving the note its sample plays untransposed at.
    pub fn set_root_key(&mut self, index: usize, note: u8) -> Result<(), Error> {
        let section = self
            .body
            .0
            .iter_mut()
            .filter(|s| s.is(section::STK))
            .nth(index)
            .ok_or_else(|| ParseError::AssertFail(format!("no stroke {index}")))?;
        stroke::set_root_key(&mut section.payload, note)?;
        Ok(())
    }

    /// Category labels, as stored in `cat`: length-prefixed strings.
    pub fn categories(&self) -> Vec<String> {
        let Some(cat) = section::find(&self.body.0, section::CAT) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut i = 0;
        while i < cat.payload.len() {
            let len = cat.payload[i] as usize;
            let from = i + 1;
            // A length running past the end means this is not a string here; the
            // section holds a few leading bytes before the labels start.
            match cat.payload.get(from..from + len) {
                Some(s) if len > 0 && s.iter().all(|&b| (0x20..0x7f).contains(&b)) => {
                    out.push(String::from_utf8_lossy(s).into_owned());
                    i = from + len;
                }
                _ => i += 1,
            }
        }
        out
    }

    fn stroke_sections(&self) -> impl Iterator<Item = &Section> {
        self.body.0.iter().filter(|s| s.is(section::STK))
    }

    fn hdr(&self) -> Result<&Section, Error> {
        section::find(&self.body.0, section::HDR)
            .ok_or_else(|| ParseError::AssertFail("no hdr section".into()).into())
    }

    fn map(&self) -> Result<&Section, Error> {
        section::find(&self.body.0, section::MAP)
            .ok_or_else(|| ParseError::AssertFail("no map section".into()).into())
    }
}
