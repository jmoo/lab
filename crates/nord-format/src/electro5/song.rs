//! The Electro 5 set list format (`.ne5t`).

use std::io::{Read, Seek, Write};

use crate::cbin::{self, BodyReader, BodyWriter, Cbin, Header};
use crate::common;
use crate::common::bank;
use crate::common::bank::Item;
use crate::electro5::program;
use crate::error::Error;
use crate::types::RangedU16Pair;

pub const FORMAT: &str = "ne5t";
/// Schema versions this build's field offsets have been validated against: 0 is the
/// eight factory demo songs, 1 is everything user-written.
pub const KNOWN_VERSIONS: &[u32] = &[0, 1];
/// The body after the container header: the 8-byte program map and 10 zero bytes.
pub const BODY_LEN: usize = 18;
/// Type-1 file length: 44-byte CBIN header + 18-byte body.
pub const FILE_LEN: usize = 0x2c + BODY_LEN;
pub const PROGRAM_COUNT: usize = 4;
pub const BANK_COUNT: u16 = 4;
pub const SLOT_COUNT: u16 = 50;

pub type Location = RangedU16Pair<BANK_COUNT, SLOT_COUNT>;
pub type Bank = bank::Bank<Song, Location>;
pub type Song = common::song::Song<PROGRAM_COUNT, Location, program::Location>;

/// The 18-byte body: four 9-bit program references packed into a big-endian u64 —
/// sub-byte fields, so this is bit territory rather than `#[bitbody]`'s byte
/// segments, and the codec is by hand.
///
/// Bits 48.. carry the version again. The container header is never transmitted
/// over USB — the device sends only this body — so the version is echoed into the
/// payload where the wire side can see it. ⚠️ It must be the *read* version, never a
/// constant: the eight factory demo songs are version 0, and stamping 1 here
/// silently rewrites them.
struct SongBody {
    version: u32,
    programs: [program::Location; PROGRAM_COUNT],
}

impl cbin::Body for SongBody {
    const LEN: Option<u64> = Some(BODY_LEN as u64);

    fn read<R: Read + Seek>(r: &mut BodyReader<'_, R>, _: &Header) -> Result<Self, Error> {
        let mut raw = [0u8; BODY_LEN];
        r.read_exact(&mut raw)?;
        let map = u64::from_be_bytes(raw[0..8].try_into().unwrap());
        let slot = |shift: u32| -> Result<program::Location, Error> {
            Ok(((map >> shift & 0b1_1111_1111) as u16).try_into()?)
        };
        Ok(SongBody {
            version: (map >> 48) as u32,
            programs: [slot(39)?, slot(30)?, slot(21)?, slot(12)?],
        })
    }

    fn write<W: Write + Seek>(&self, w: &mut BodyWriter<'_, W>) -> Result<(), Error> {
        let [a, b, c, d] = self.programs;
        let map = (self.version as u64) << 48
            | (a.as_u16() as u64) << 39
            | (b.as_u16() as u64) << 30
            | (c.as_u16() as u64) << 21
            | (d.as_u16() as u64) << 12;
        w.write_all(&map.to_be_bytes())?;
        w.write_all(&[0u8; BODY_LEN - 8])?;
        Ok(())
    }
}

impl Song {
    pub fn read_from(reader: &mut (impl Read + Seek)) -> Result<Song, Error> {
        let file: Cbin<SongBody> = cbin::read(reader, FORMAT)?;
        program::known_version(FORMAT, file.header.version, KNOWN_VERSIONS)?;

        let mut song = Song::new(program::location(&file.header)?, file.body.programs);
        song.set_version(file.header.version);
        song.set_generation(file.header.generation);
        Ok(song)
    }

    pub fn write_to(&self, writer: &mut (impl Write + Seek)) -> Result<(), Error> {
        let mut header = Header::new(FORMAT, self.location().inner(), self.version());
        header.generation = self.generation();
        let file = Cbin {
            header,
            body: SongBody {
                version: self.version(),
                programs: *self.programs(),
            },
        };
        file.write_to(writer)
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
    /// version 1. A writer stamping a constant into the header or the map's echo
    /// silently promotes them — a real difference at offset `0x14` and again in the
    /// body, on every one of the eight.
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
