pub mod bits;
pub mod common;
pub mod crc;
pub mod electro5;
pub mod error;
pub mod file;
pub mod panel;
pub mod types;
pub mod util;

pub use file::{File, Format};

use crate::common::sample::Sample;
use crate::common::{piano, sample};
use std::io::{BufReader, Read, Seek};
use std::path::Path;
use util::{peek, FileType};

use crate::error::{Error, ParseError};

#[cfg(feature = "bundle")]
#[derive(Debug)]
pub enum Bundle {
    Electro5(electro5::Bundle),
}

#[derive(Debug)]
pub enum Program {
    Electro5(electro5::Program),
}

/// The live buffer — the panel as it stands, not a saved program. Same body as
/// [`Program`], under its own format tag.
#[derive(Debug)]
pub enum Live {
    Electro5(electro5::Live),
}

#[derive(Debug)]
pub enum Song {
    Electro5(electro5::Song),
}

#[derive(Debug)]
pub enum Settings {
    Electro5(electro5::Settings),
}

/// One decoded file.
///
/// `Program` is much the largest variant: a decoded panel holds its fields *and* the
/// bytes it came from, and the organ panel's bytes alone are 69. Left unboxed — one of
/// these exists per file being read, never in a collection.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Entity {
    Song(Song),
    Program(Program),
    Live(Live),
    Piano(piano::Piano),
    Settings(Settings),
    Sample(Sample),
    #[cfg(feature = "bundle")]
    Bundle(Bundle),
}

pub fn from_stream(reader: &mut (impl Read + Seek + Sized)) -> Result<Entity, Error> {
    let header = peek(reader)?;

    match header.file_type {
        #[cfg(feature = "bundle")]
        FileType::Zip => Ok(Entity::Bundle(Bundle::Electro5(
            electro5::Bundle::read_from(reader)?,
        ))),
        #[cfg(not(feature = "bundle"))]
        FileType::Zip => {
            Err(ParseError::UnknownFileType("zip (bundle feature disabled)".to_string()).into())
        }
        FileType::Cbin => match header.format.as_str() {
            sample::FORMAT => Ok(Entity::Sample(sample::Sample::read_from(reader)?)),
            piano::FORMAT => Ok(Entity::Piano(piano::Piano::read_from(reader)?)),
            electro5::song::FORMAT => Ok(Entity::Song(Song::Electro5(electro5::Song::read_from(
                reader,
            )?))),
            electro5::program::FORMAT => Ok(Entity::Program(Program::Electro5(
                electro5::Program::read_from(reader)?,
            ))),
            electro5::live::FORMAT => Ok(Entity::Live(Live::Electro5(electro5::Live::read_from(
                reader,
            )?))),
            electro5::settings::FORMAT => Ok(Entity::Settings(Settings::Electro5(
                electro5::Settings::read_from(reader)?,
            ))),
            e => Err(ParseError::UnknownFormat(e.to_string()).into()),
        },
        e => Err(ParseError::UnknownFileType(e.as_str().to_string()).into()),
    }
}

/// One decoded entity from a bare wire body — the device transfers only bodies, so
/// this is the read path that never fabricates a container to immediately re-parse it.
///
/// `format` and `version` are the tag and schema version the device reported for the
/// slot, and `location` is the raw wire form of the slot it came from. Formats the
/// wire does not transfer this way (bundles) are refused as unknown.
pub fn from_wire(format: &str, location: u32, version: u32, body: &[u8]) -> Result<Entity, Error> {
    use crate::file::Location;

    fn at<L: Location>(location: u32) -> Result<L, Error> {
        Ok(L::from_wire(location)?)
    }

    match format {
        sample::FORMAT => Ok(Entity::Sample(sample::Sample::from_wire(
            at(location)?,
            version,
            body,
        )?)),
        piano::FORMAT => Ok(Entity::Piano(piano::Piano::from_wire(
            at(location)?,
            version,
            body,
        )?)),
        electro5::song::FORMAT => Ok(Entity::Song(Song::Electro5(electro5::Song::from_wire(
            at(location)?,
            version,
            body,
        )?))),
        electro5::program::FORMAT => Ok(Entity::Program(Program::Electro5(
            electro5::Program::from_wire(at(location)?, version, body)?,
        ))),
        electro5::live::FORMAT => Ok(Entity::Live(Live::Electro5(electro5::Live::from_wire(
            at(location)?,
            version,
            body,
        )?))),
        electro5::settings::FORMAT => Ok(Entity::Settings(Settings::Electro5(
            electro5::Settings::from_wire(at(location)?, version, body)?,
        ))),
        e => Err(ParseError::UnknownFormat(e.to_string()).into()),
    }
}

pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Entity, Error> {
    from_stream(&mut BufReader::new(std::fs::File::open(path)?))
}

/// Serialise an [`Entity`] back to the bytes of its file — the counterpart to
/// [`from_stream`].
///
/// Every concrete type has a `to_bytes`, but the enum needs a way out too, so a caller
/// holding an `Entity` can round-trip it without re-matching every variant. A
/// fixed-length format's emitted length is checked inside [`File`]'s writer against
/// what its module declares.
///
/// For every format this crate decodes, `to_bytes(from_stream(x)) == x` byte-for-byte.
/// That is the crate's central invariant — decoded values are read-only views over a
/// verbatim body, so a re-emit cannot drift — and `nord verify` exists to check it
/// against real specimens.
///
/// Bundles are unsupported: [`electro5::Bundle`] is a ZIP walk over other entities,
/// not a re-emittable structure.
pub fn to_bytes(entity: &Entity) -> Result<Vec<u8>, Error> {
    match entity {
        Entity::Program(Program::Electro5(p)) => p.to_bytes(),
        Entity::Live(Live::Electro5(l)) => l.to_bytes(),
        Entity::Song(Song::Electro5(s)) => s.to_bytes(),
        Entity::Settings(Settings::Electro5(s)) => s.to_bytes(),
        Entity::Piano(p) => p.to_bytes(),
        Entity::Sample(s) => s.to_bytes(),
        #[cfg(feature = "bundle")]
        Entity::Bundle(_) => Err(ParseError::UnknownFormat(
            "bundle (an archive, not a re-emittable entity)".to_string(),
        )
        .into()),
    }
}
