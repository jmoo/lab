use binrw::{binrw, BinRead, BinWriterExt};
use std::io::{Cursor, Read, Seek, Write};

use crate::common;
use crate::common::bank;
use crate::common::bank::Item;
use crate::common::container;
use crate::error::Error;

use crate::electro5::program;

use crate::types::RangedU16Pair;

pub const FORMAT: &str = "ne5t";
/// Schema versions this build's field offsets have been validated against: 0 is the
/// eight factory demo songs, 1 is everything user-written.
pub const KNOWN_VERSIONS: &[u32] = &[0, 1];
/// The 18-byte body: the four program references, and the run of zeros after them.
pub const BODY_LEN: usize = 18;
/// Total file length: 44-byte CBIN header + the body.
pub const FILE_LEN: usize = container::HEADER_LEN + BODY_LEN;
pub const PROGRAM_COUNT: usize = 4;
pub const BANK_COUNT: u16 = 4;
pub const SLOT_COUNT: u16 = 50;

pub type Location = RangedU16Pair<BANK_COUNT, SLOT_COUNT>;
pub type Bank = bank::Bank<Song, Location>;
pub type Song = common::song::Song<PROGRAM_COUNT, Location, program::Location>;

/// The song body. Offsets are absolute in a type-1 file, as everywhere in this crate;
/// the body itself starts at [`container::HEADER_LEN`].
///
/// `version` is a write import rather than a field: the body only *echoes* the version,
/// which the container holds.
#[binrw]
#[bw(import(version: u32))]
struct Schema {
    // 0x2c..0x34. Bits 48.. carry the version again. The container header is never
    // transmitted over USB — the device sends only this body — so the version is echoed
    // into the payload where the wire side can see it. ⚠️ It must be the version the
    // container carries, never a constant: the eight factory demo songs are version 0,
    // and stamping 1 here silently rewrites them.
    #[brw(big)]
    #[bw(calc = (
    ((* a).as_u16() as u64) << 39
    | ((* b).as_u16() as u64) << 30
    | ((* c).as_u16() as u64) << 21
    | ((* d).as_u16() as u64) << 12)
    | ((version as u64) << 48)
    )]
    map: u64,

    /// These bytes are part of the crc check so they cannot be skipped with the pad_after directive
    #[bw(calc = [0; 10])]
    pad: [u8; 10],

    #[br(try_calc = ((map >> 39 & 0b111111111) as u16).try_into())]
    #[bw(ignore)]
    pub a: program::Location,

    #[br(try_calc = ((map >> 30 & 0b111111111) as u16).try_into())]
    #[bw(ignore)]
    pub b: program::Location,

    #[br(try_calc = ((map >> 21 & 0b111111111) as u16).try_into())]
    #[bw(ignore)]
    pub c: program::Location,

    #[br(try_calc = ((map >> 12 & 0b111111111) as u16).try_into())]
    #[bw(ignore)]
    pub d: program::Location,
}

impl Song {
    pub fn read_from(reader: &mut (impl Read + Seek)) -> Result<Song, Error> {
        let (header, location, body) =
            container::Container::open_fixed(reader, FORMAT, KNOWN_VERSIONS, FILE_LEN)?;
        let schema = Schema::read_be(&mut Cursor::new(body))?;

        let mut song = Song::new(location, [schema.a, schema.b, schema.c, schema.d]);
        song.set_header(header);
        Ok(song)
    }

    pub fn write_to(&self, writer: &mut (impl Write + Seek)) -> Result<(), Error> {
        let schema = Schema {
            a: self.programs()[0],
            b: self.programs()[1],
            c: self.programs()[2],
            d: self.programs()[3],
        };

        let mut body = Cursor::new(Vec::new());
        body.write_be_args(&schema, (self.version(),))?;

        // The song carries everything but the tag; only this module knows which format
        // it is.
        let mut header = self.header().clone();
        header.tag = FORMAT.to_string();
        container::Container {
            header,
            location: container::location_of(self.location().x(), self.location().y()),
            body: body.into_inner(),
        }
        .write_to(writer)
    }
}

#[cfg(test)]
mod tests {
    use super::Song;
    use crate::common::bank::Item;
    use crate::error::Error;
    use std::io::Cursor;

    #[test]
    fn read_write_new_song() -> Result<(), Error> {
        let song = Song::new(
            (0, 1).try_into()?,
            [
                (1, 2).try_into()?,
                (2, 3).try_into()?,
                (3, 4).try_into()?,
                (4, 5).try_into()?,
            ],
        );

        // Assert song was created with correct values
        assert_eq!(song.location(), (0, 1));
        assert_eq!(song.get(0), (1, 2));
        assert_eq!(song.get(1), (2, 3));
        assert_eq!(song.get(2), (3, 4));
        assert_eq!(song.get(3), (4, 5));

        // Read/Write song to result
        let mut write_result = Vec::new();
        song.write_to(&mut Cursor::new(&mut write_result)).unwrap();

        let result = Song::read_from(&mut Cursor::new(&mut write_result)).unwrap();

        // Assert those values are the same after writing and reading
        assert_eq!(song.location(), result.location());
        assert_eq!(song.get(0), result.get(0));
        assert_eq!(song.get(1), result.get(1));
        assert_eq!(song.get(2), result.get(2));
        assert_eq!(song.get(3), result.get(3));

        Ok(())
    }

    /// A version-0 song must come back out as version 0.
    ///
    /// The eight factory demo songs are version 0 and everything user-written is
    /// version 1. The writer used to hardcode `version: 1` and a constant `1 << 48` in
    /// the map word, so re-emitting a factory song silently promoted it — a real
    /// difference at offset `0x14` and again in the body, on every one of the eight.
    #[test]
    fn version_survives_a_round_trip() -> Result<(), Error> {
        for version in [0u32, 1] {
            let mut song = Song::new(
                (0, 5).try_into()?,
                [
                    (1, 2).try_into()?,
                    (2, 3).try_into()?,
                    (3, 4).try_into()?,
                    (4, 5).try_into()?,
                ],
            );
            song.set_version(version);

            let mut bytes = Vec::new();
            song.write_to(&mut Cursor::new(&mut bytes)).unwrap();

            // Header field at 0x14, little-endian.
            assert_eq!(
                u32::from_le_bytes(bytes[0x14..0x18].try_into().unwrap()),
                version,
                "header version for v{version}",
            );
            // ...and the echo in the top bits of the big-endian map word at 0x2c, which
            // is the only copy the device ever sees.
            assert_eq!(
                u16::from_be_bytes(bytes[0x2c..0x2e].try_into().unwrap()) as u32,
                version,
                "body version echo for v{version}",
            );

            let back = Song::read_from(&mut Cursor::new(&mut bytes)).unwrap();
            assert_eq!(back.version(), version);
            assert_eq!(back.get(0), song.get(0));
        }
        Ok(())
    }

    #[test]
    fn update_song_program() -> Result<(), Error> {
        let mut song = Song::new(
            (0, 1).try_into()?,
            [
                (1, 2).try_into()?,
                (2, 3).try_into()?,
                (3, 4).try_into()?,
                (4, 5).try_into()?,
            ],
        );

        // Update program 1
        song.set(1, (5, 20).try_into()?);

        // Assert song was updated with correct values
        assert_eq!(song.location(), (0, 1));
        assert_eq!(song.get(0), (1, 2));
        assert_eq!(song.get(1), (5, 20));
        assert_eq!(song.get(2), (3, 4));
        assert_eq!(song.get(3), (4, 5));

        // Read/Write song to result
        let mut write_result = Vec::new();
        song.write_to(&mut Cursor::new(&mut write_result)).unwrap();

        let result = Song::read_from(&mut Cursor::new(&mut write_result)).unwrap();

        // Assert those values are the same after writing and reading
        assert_eq!(song.location(), result.location());
        assert_eq!(song.get(0), result.get(0));
        assert_eq!(song.get(1), result.get(1));
        assert_eq!(song.get(2), result.get(2));
        assert_eq!(song.get(3), result.get(3));

        Ok(())
    }
}
