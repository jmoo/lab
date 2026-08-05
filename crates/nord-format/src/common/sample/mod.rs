//! Sample instruments (`.nsmp`) — the Nord Sample Library format.
//!
//! Shared across the Nord line rather than specific to one model, which is why this sits
//! in [`crate::common`]. The body inside the usual [`container`] is a chain of tagged
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

use crate::common::container::{self, Container};
use crate::error::{Error, ParseError};
use std::fmt;
use std::io::{Read, Seek, Write};

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

/// A sample instrument.
///
/// Sections are held in file order, including repeats — `stk` appears once per zone.
pub struct Sample {
    /// The container header as read. Its `version` is content as
    /// `format * 100 + revision`, so a 2.0 instrument reads 200 — not a schema revision
    /// and not gated, since it moves with the library's own content.
    ///
    /// ⚠️ Its `trailer` is `0x000f0000` on every specimen, where every slotted format
    /// holds [`container::SLOT_TRAILER`]. Meaning unknown; preserved verbatim.
    pub header: container::Header,

    /// Where a slotted format holds bank and slot. `0xFFFFFFFF` on every specimen — a
    /// library sample has no slot until an instrument gives it one.
    pub location: u32,

    pub sections: Vec<Section>,
}

impl Sample {
    /// Reads a whole instrument, verifying its checksum.
    pub fn read_from(reader: &mut (impl Read + Seek)) -> Result<Sample, Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Sample::from_bytes(&bytes)
    }

    /// Decodes an instrument of either CBIN generation.
    pub fn from_bytes(bytes: &[u8]) -> Result<Sample, Error> {
        let file = Container::parse(bytes)?;
        if file.header.tag != FORMAT {
            return Err(ParseError::WrongFormat {
                expected: FORMAT,
                got: file.header.tag,
            }
            .into());
        }
        let sections = section::read_chain(&file.body, 0)?;
        Ok(Sample {
            header: file.header,
            location: file.location,
            sections,
        })
    }

    pub fn write_to(&self, writer: &mut (impl Write + Seek)) -> Result<(), Error> {
        self.container().write_to(writer)
    }

    /// Serialises, recomputing the checksum over the body it just produced.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        self.container().to_bytes()
    }

    /// The instrument as its container: the header it was read with, around the section
    /// chain as it now stands.
    fn container(&self) -> Container {
        let mut body = Vec::new();
        for s in &self.sections {
            s.write_into(&mut body);
        }
        Container {
            header: self.header.clone(),
            location: self.location,
            body,
        }
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
        let hdr = section::find_mut(&mut self.sections, section::HDR)
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
        let map = section::find_mut(&mut self.sections, section::MAP)
            .ok_or_else(|| ParseError::AssertFail("no map section".into()))?;
        zone::set_top_note(&mut map.payload, index, note)?;
        Ok(())
    }

    /// One stroke per zone, in the same high-to-low order as [`Sample::zones`].
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
            .sections
            .iter_mut()
            .filter(|s| s.is(section::STK))
            .nth(index)
            .ok_or_else(|| ParseError::AssertFail(format!("no stroke {index}")))?;
        stroke::set_root_key(&mut section.payload, note)?;
        Ok(())
    }

    /// Category labels, as stored in `cat`: length-prefixed strings.
    pub fn categories(&self) -> Vec<String> {
        let Some(cat) = section::find(&self.sections, section::CAT) else {
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
        self.sections.iter().filter(|s| s.is(section::STK))
    }

    fn hdr(&self) -> Result<&Section, Error> {
        section::find(&self.sections, section::HDR)
            .ok_or_else(|| ParseError::AssertFail("no hdr section".into()).into())
    }

    fn map(&self) -> Result<&Section, Error> {
        section::find(&self.sections, section::MAP)
            .ok_or_else(|| ParseError::AssertFail("no map section".into()).into())
    }
}

impl fmt::Debug for Sample {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sample")
            .field("name", &self.name().unwrap_or_default())
            .field("version", &self.header.version)
            .field("zones", &self.zones().map(|z| z.len()).unwrap_or(0))
            .field("sections", &self.sections)
            .finish()
    }
}
