use crate::common::container::Header;
use crate::error::Error;
use crate::file::{sealed, BodyReader, File, Format, Opaque, Verbatim};
use std::io::{Seek, Write};

pub const FORMAT: &str = "npno";

/// The `npno` format: a piano library file.
///
/// Nothing below the CBIN header is mapped, so the body is held verbatim: enough to
/// identify a file and hand it back unchanged, and no more. The header's version and
/// location fields are library content of unknown meaning, preserved rather than
/// checked.
#[derive(Debug)]
pub struct Npno;

impl sealed::Sealed for Npno {}

impl Format for Npno {
    const TAG: &'static str = FORMAT;
    const KNOWN_VERSIONS: &'static [u32] = &[];
    const FILE_LEN: Option<usize> = None;
    type Location = Verbatim;
    type Body = Opaque;

    fn read_body(r: &mut BodyReader, _header: &Header) -> Result<Opaque, Error> {
        Ok(Opaque(r.bytes()?))
    }

    fn write_body(
        body: &Opaque,
        _header: &Header,
        w: &mut (impl Write + Seek),
    ) -> Result<(), Error> {
        w.write_all(&body.0)?;
        Ok(())
    }

    // A library file, not a slot save: its trailer is its own, preserved verbatim.
    fn check(_header: &Header) -> Result<(), crate::error::ParseError> {
        Ok(())
    }
}

pub type Piano = File<Npno>;

impl File<Npno> {
    pub fn new() -> Piano {
        File {
            header: Header::new(FORMAT, 0),
            location: Verbatim(0),
            body: Opaque(Vec::new()),
        }
    }
}

impl Default for File<Npno> {
    fn default() -> Self {
        Self::new()
    }
}
