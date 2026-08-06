//! The CBIN container, in both of its header generations.
//!
//! Every file this crate reads opens with `CBIN` and a little-endian u32 the corpus
//! calls the header **type**. Two values occur, and they change the container without
//! changing the body:
//!
//! ```text
//! type 1   0x00 "CBIN"   0x04 type   0x08 tag   0x0c location   0x10 trailer
//!          0x14 version  0x18 crc32  0x1c 16 unclaimed bytes    0x2c body … EOF
//!
//! type 0   0x00 "CBIN"   0x04 type   0x08 tag   0x0c location   0x10 trailer
//!          0x14 version  0x18 body … EOF-2     EOF-2 crc16
//! ```
//!
//! So the same content is exactly [`SIZE_DELTA`] bytes shorter as type 0, and the two
//! checksums cover different things: the type-1 crc32 covers the body alone, the type-0
//! crc16 covers the whole file up to itself, header included.
//!
//! Inferred from specimens; not confirmed on hardware. The evidence is that every
//! type-0 file in the corpus — across `.ne5p`, `.ne5l`, `.ne5s`, `.ne5t`, `.ns2p`,
//! `.ns2s`, `.ns2y`, `.ns2l`, `.nspg`, `.nss`, `.nsmp` and `.nsmp3` — verifies under
//! these parameters, every type-1 file verifies under its own, and each Electro 5
//! format's type-0 files are its type-1 length less [`SIZE_DELTA`] to the byte.
//!
//! ## How the rest of the crate sees it
//!
//! [`Container`] is the only thing that knows any of the above. It parses either
//! generation, verifies that generation's own checksum, and re-emits in the generation
//! the file arrived in with the checksum recomputed over the bytes it actually wrote.
//! A format schema is the **body** — offsets from the body's first byte, no header,
//! no checksum field — and takes its tag, slot, version and generation from here.

use crate::crc::{Crc16, Crc32, CrcWriter, MultipartCrc16, MultipartCrc32, Width};
use crate::error::{Error, ParseError};
use std::io::{Cursor, Seek, Write};

/// Length of a type-1 header: through the crc32 and the 16 unclaimed bytes after it.
pub const HEADER_LEN: usize = 0x2c;

/// Length of a type-0 header: the same fields through `version`, and then the body.
pub const SHORT_HEADER_LEN: usize = 0x18;

/// A type-0 file's trailing crc16.
pub const CRC16_LEN: usize = 2;

/// How much shorter a type-0 file is than the same content as type 1.
pub const SIZE_DELTA: usize = HEADER_LEN - SHORT_HEADER_LEN - CRC16_LEN;

/// The header type, as both generations store it.
pub const TYPE_AT: usize = 0x04;

/// The short header with the trailing crc16.
pub const TYPE_SHORT: u32 = 0;

/// The long header with the crc32 in it.
pub const TYPE_LONG: u32 = 1;

/// The `0x10` field of every format addressed by slot. A library sample holds something
/// else there — see [`crate::common::sample::Sample`].
pub const SLOT_TRAILER: u32 = 0xFFFFFFFF;

/// Where a type-1 header keeps its crc32.
const CRC32_AT: usize = 0x18;

/// The header type of a CBIN file, from as little as its first twelve bytes.
pub fn header_type(bytes: &[u8]) -> Result<u32, ParseError> {
    if bytes.len() < TYPE_AT + 4 {
        return Err(ParseError::AssertFail(format!(
            "{} bytes is too short to hold a CBIN header type",
            bytes.len()
        )));
    }
    if &bytes[0..4] != b"CBIN" {
        return Err(ParseError::UnknownFormat(
            String::from_utf8_lossy(&bytes[0..4]).into_owned(),
        ));
    }
    Ok(u32::from_le_bytes(
        bytes[TYPE_AT..TYPE_AT + 4].try_into().unwrap(),
    ))
}

/// Bytes of header ahead of the body in the given generation.
pub fn body_at(header_type: u32) -> usize {
    match header_type {
        TYPE_SHORT => SHORT_HEADER_LEN,
        _ => HEADER_LEN,
    }
}

/// What a fixed-length format occupies on disk in the given generation, from the
/// type-1 length its module declares.
pub fn stored_len(header_type: u32, type1_len: usize) -> usize {
    match header_type {
        TYPE_SHORT => type1_len - SIZE_DELTA,
        _ => type1_len,
    }
}

/// A slot as a header stores it at `0x0c`: bank then slot, two little-endian `u16`s.
pub fn location_of(bank: u16, slot: u16) -> u32 {
    bank as u32 | (slot as u32) << 16
}

/// The bytes a generation's checksum covers, as [`MultipartCrc32`] takes them.
///
/// ⚠️ The second element is the region's last byte index minus the first — the inclusive
/// span [`crate::crc::MultipartCrc`] is written against, not a byte count. Read and write
/// both come here so they cannot disagree about which bytes are checksummed.
fn region(header_type: u32, body_len: usize) -> (u64, u64) {
    match header_type {
        TYPE_SHORT => (0, (SHORT_HEADER_LEN + body_len).saturating_sub(1) as u64),
        _ => (HEADER_LEN as u64, body_len.saturating_sub(1) as u64),
    }
}

/// A CBIN header's format-independent fields: everything but the slot, which each format
/// addresses in its own space and so keeps typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// The generation this file arrived in, [`TYPE_SHORT`] or [`TYPE_LONG`].
    ///
    /// ⚠️ Carried so a file goes back out as the generation it came in. The two are
    /// [`SIZE_DELTA`] bytes apart and checksum different regions, so a type-0 file
    /// written back as type 1 is a different file.
    pub header_type: u32,

    /// The four-character format tag at `0x08`.
    pub tag: String,

    /// `0x10`. [`SLOT_TRAILER`] in every format addressed by slot; `0x000f0000` on every
    /// sample specimen, meaning unknown.
    pub trailer: u32,

    /// `0x14`. What each format module calls its schema version.
    pub version: u32,

    /// `0x1c..0x2c`, which no field of any format this build reads claims. Preserved
    /// verbatim rather than assumed.
    ///
    /// ⚠️ A type-0 header has no room for these, so writing one drops them.
    pub unclaimed: [u8; 16],
}

impl Header {
    /// A type-1 header for a format addressed by slot — the generation the instrument
    /// writes.
    pub fn new(tag: &str, version: u32) -> Header {
        Header {
            header_type: TYPE_LONG,
            tag: tag.to_string(),
            trailer: SLOT_TRAILER,
            version,
            unclaimed: [0; 16],
        }
    }
}

/// One CBIN file: its header, the slot it addresses, and the body between them.
///
/// Reading verifies the checksum of whichever generation the file arrived in; writing
/// recomputes it over the bytes actually emitted. Nothing else in the crate touches
/// either checksum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    pub header: Header,

    /// `0x0c`. Bank and slot as two little-endian `u16`s in every format addressed by
    /// slot; `0xFFFFFFFF` in a library sample, which has no slot until an instrument
    /// gives it one.
    pub location: u32,

    /// Everything between the header and the checksum: from `0x2c` in a type-1 file,
    /// from `0x18` to the trailing crc16 in a type-0 one.
    pub body: Vec<u8>,
}

impl Container {
    /// A type-1 container around `body`, tagged and addressed.
    pub fn new(tag: &str, location: u32, version: u32, body: Vec<u8>) -> Container {
        Container {
            header: Header::new(tag, version),
            location,
            body,
        }
    }

    /// Bank, as a slotted format's header stores it.
    pub fn bank(&self) -> u16 {
        self.location as u16
    }

    /// Slot, as a slotted format's header stores it.
    pub fn slot(&self) -> u16 {
        (self.location >> 16) as u16
    }

    /// Parse one whole CBIN file, verifying the checksum of its generation.
    ///
    /// Refuses a generation this build has no layout for: guessing which bytes are
    /// header would decode the body at the wrong offset and still produce plausible
    /// values.
    pub fn parse(bytes: &[u8]) -> Result<Container, ParseError> {
        let header_type = header_type(bytes)?;
        let (head_len, sum_len) = match header_type {
            TYPE_LONG => (HEADER_LEN, 0),
            TYPE_SHORT => (SHORT_HEADER_LEN, CRC16_LEN),
            other => return Err(ParseError::UnknownHeaderType(other)),
        };
        if bytes.len() < head_len + sum_len {
            return Err(ParseError::AssertFail(format!(
                "{} bytes is shorter than a type-{header_type} header and its checksum",
                bytes.len()
            )));
        }

        let body = bytes[head_len..bytes.len() - sum_len].to_vec();
        verify(header_type, bytes, body.len())?;

        let le = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        Ok(Container {
            header: Header {
                header_type,
                tag: String::from_utf8_lossy(&bytes[0x08..0x0c]).into_owned(),
                trailer: le(0x10),
                version: le(0x14),
                unclaimed: match header_type {
                    TYPE_LONG => bytes[0x1c..HEADER_LEN].try_into().unwrap(),
                    _ => [0; 16],
                },
            },
            location: le(0x0c),
            body,
        })
    }

    /// Emit the file, in its own generation, with the checksum computed over the bytes
    /// as they go out.
    pub fn write_to(&self, writer: &mut (impl Write + Seek)) -> Result<(), Error> {
        let fields = self.fields()?;
        let (first, length) = region(self.header.header_type, self.body.len());
        match self.header.header_type {
            TYPE_LONG => {
                let mut w = CrcWriter::<_, Crc32>::new(writer, first, length);
                w.write_all(&fields)?;
                // The crc32 precedes the body it covers, so this is a placeholder and
                // the writer patches it once the body has gone through.
                let sum = w.checksum()?;
                Crc32::write_le(sum, &mut w)?;
                w.write_all(&self.header.unclaimed)?;
                w.write_all(&self.body)?;
            }
            TYPE_SHORT => {
                let mut w = CrcWriter::<_, Crc16>::new(writer, first, length);
                w.write_all(&fields)?;
                w.write_all(&self.body)?;
                let sum = w.checksum()?;
                Crc16::write_le(sum, &mut w)?;
            }
            other => return Err(ParseError::UnknownHeaderType(other).into()),
        }
        Ok(())
    }

    /// The file's bytes — [`Container::write_to`] into memory.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut out = Cursor::new(Vec::new());
        self.write_to(&mut out)?;
        Ok(out.into_inner())
    }

    /// The header fields both generations share, `0x00..0x18`.
    fn fields(&self) -> Result<[u8; SHORT_HEADER_LEN], ParseError> {
        // The tag field is four bytes wide, so a tag of any other length would shift
        // every field after it.
        let tag = self.header.tag.as_bytes();
        if tag.len() != 4 {
            return Err(ParseError::AssertFail(format!(
                "CBIN tag {:?} is {} bytes, not 4",
                self.header.tag,
                tag.len()
            )));
        }

        let mut out = [0u8; SHORT_HEADER_LEN];
        out[0x00..0x04].copy_from_slice(b"CBIN");
        out[0x04..0x08].copy_from_slice(&self.header.header_type.to_le_bytes());
        out[0x08..0x0c].copy_from_slice(tag);
        out[0x0c..0x10].copy_from_slice(&self.location.to_le_bytes());
        out[0x10..0x14].copy_from_slice(&self.header.trailer.to_le_bytes());
        out[0x14..0x18].copy_from_slice(&self.header.version.to_le_bytes());
        Ok(out)
    }
}

/// Check the file's stored checksum against the bytes it arrived with.
fn verify(header_type: u32, bytes: &[u8], body_len: usize) -> Result<(), ParseError> {
    let (first, length) = region(header_type, body_len);
    match header_type {
        TYPE_LONG => {
            let stored = u32::from_le_bytes(bytes[CRC32_AT..CRC32_AT + 4].try_into().unwrap());
            let mut crc = MultipartCrc32::new(first, length);
            crc.update(0, bytes);
            let computed = crc.checksum();
            if computed != stored {
                return Err(ParseError::AssertFail(format!(
                    "type-1 CBIN: stored checksum {stored:#010x} does not match the body's \
                     {computed:#010x}"
                )));
            }
        }
        _ => {
            let at = bytes.len() - CRC16_LEN;
            let stored = u16::from_le_bytes(bytes[at..].try_into().unwrap());
            let mut crc = MultipartCrc16::new(first, length);
            crc.update(0, bytes);
            let computed = crc.checksum();
            if computed != stored {
                return Err(ParseError::AssertFail(format!(
                    "type-0 CBIN: stored checksum {stored:#06x} does not match the file's \
                     {computed:#06x}"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_1() -> Vec<u8> {
        let mut c = Container::new("ne5p", location_of(3, 7), 4, vec![1, 2, 3, 4, 5, 6, 7, 8]);
        c.header.unclaimed = [0xab; 16];
        c.to_bytes().unwrap()
    }

    fn type_0() -> Vec<u8> {
        let mut c = Container::new("ne5p", location_of(3, 7), 4, vec![1, 2, 3, 4, 5, 6, 7, 8]);
        c.header.header_type = TYPE_SHORT;
        c.to_bytes().unwrap()
    }

    /// Both generations parse to the same header fields and the same body, and go back
    /// out as the file they came from.
    #[test]
    fn either_generation_round_trips_byte_for_byte() {
        for file in [type_1(), type_0()] {
            let c = Container::parse(&file).unwrap();
            assert_eq!(c.header.tag, "ne5p");
            assert_eq!((c.bank(), c.slot()), (3, 7));
            assert_eq!(c.header.version, 4);
            assert_eq!(c.body, vec![1, 2, 3, 4, 5, 6, 7, 8]);
            assert_eq!(c.to_bytes().unwrap(), file);
        }
        assert_eq!(type_1().len(), type_0().len() + SIZE_DELTA);
    }

    /// The 16 bytes no format claims are the file's, not this crate's to invent.
    #[test]
    fn the_unclaimed_bytes_survive_verbatim() {
        let file = type_1();
        assert_eq!(&file[0x1c..HEADER_LEN], &[0xab; 16]);
        assert_eq!(
            Container::parse(&file).unwrap().header.unclaimed,
            [0xab; 16]
        );
    }

    /// Each generation is checked against its own checksum — the crc32 over the body,
    /// the crc16 over the whole file — so a corrupted file is refused either way.
    #[test]
    fn a_corrupted_file_is_refused_in_both_generations() {
        for mut file in [type_1(), type_0()] {
            let body_at = body_at(header_type(&file).unwrap());
            file[body_at] ^= 0xff;
            let err = Container::parse(&file).expect_err("a corrupted body must not parse");
            assert!(err.to_string().contains("checksum"), "{err}");
        }
    }

    /// A type-0 crc16 covers the header too, so a header edit invalidates it just as a
    /// body edit does. Nothing else in the crate checks the header's bytes.
    #[test]
    fn a_type_0_checksum_covers_its_header() {
        let mut file = type_0();
        file[0x08] ^= 0xff;
        assert!(Container::parse(&file).is_err());
    }

    /// A generation this build has no layout for is refused rather than decoded on a
    /// guess about where its body starts.
    #[test]
    fn an_unknown_header_type_is_refused() {
        let mut file = b"CBIN\x02\x00\x00\x00ne5p".to_vec();
        file.resize(HEADER_LEN + 8, 0);
        assert!(matches!(
            Container::parse(&file),
            Err(ParseError::UnknownHeaderType(2))
        ));
    }
}
