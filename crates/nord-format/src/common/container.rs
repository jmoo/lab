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
//! It does not. [`widen`] turns a type-0 file into the type-1 image the format readers
//! are written against, [`narrow`] puts it back, and every reader keeps the header type
//! it read so a file is re-emitted in the generation it arrived in.
//!
//! ⚠️ **The crc32 in a widened image is synthesised**, so a format's own
//! `assert(checksum == crc32)` is vacuous for a type-0 file. [`widen`] verifying the
//! crc16 is what stands in its place — weaken that and type-0 files stop being checked
//! at all.

use crate::crc::{crc16, crc32};
use crate::error::{Error, ParseError};
use std::io::{Read, Seek, SeekFrom};

/// Length of a type-1 header: through the crc32 and the 16 unclaimed bytes after it.
pub const HEADER_LEN: usize = 0x2c;

/// Length of a type-0 header: the same fields through `version`, and then the body.
pub const SHORT_HEADER_LEN: usize = 0x18;

/// A type-0 file's trailing crc16.
pub const TRAILER_LEN: usize = 2;

/// How much shorter a type-0 file is than the same content as type 1.
pub const SIZE_DELTA: usize = HEADER_LEN - SHORT_HEADER_LEN - TRAILER_LEN;

/// The header type, as both generations store it.
pub const TYPE_AT: usize = 0x04;

/// The short header with the trailing crc16.
pub const TYPE_SHORT: u32 = 0;

/// The long header with the crc32 in it.
pub const TYPE_LONG: u32 = 1;

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

/// What a fixed-length format occupies on disk in the given generation, from the
/// type-1 length its module declares.
pub fn stored_len(header_type: u32, type1_len: usize) -> usize {
    match header_type {
        TYPE_SHORT => type1_len - SIZE_DELTA,
        _ => type1_len,
    }
}

/// One CBIN file's bytes as the type-1 image the format readers expect.
///
/// A type-1 file passes through. A type-0 file has its trailing crc16 verified and
/// dropped, and the crc32 and 16 unclaimed bytes a type-1 header carries synthesised in
/// their place.
///
/// Refuses a generation this build has no layout for: guessing which bytes are header
/// would decode the body at the wrong offset and still produce plausible values.
pub fn widen(bytes: &[u8]) -> Result<Vec<u8>, ParseError> {
    match header_type(bytes)? {
        TYPE_LONG => Ok(bytes.to_vec()),
        TYPE_SHORT => {
            if bytes.len() < SHORT_HEADER_LEN + TRAILER_LEN {
                return Err(ParseError::AssertFail(format!(
                    "{} bytes is shorter than a type-0 header and its checksum",
                    bytes.len()
                )));
            }
            let (checked, stored) = bytes.split_at(bytes.len() - TRAILER_LEN);
            let stored = u16::from_le_bytes(stored.try_into().unwrap());
            let computed = crc16(checked);
            if computed != stored {
                return Err(ParseError::AssertFail(format!(
                    "type-0 CBIN: stored checksum {stored:#06x} does not match the file's \
                     {computed:#06x}"
                )));
            }

            let body = &checked[SHORT_HEADER_LEN..];
            let mut out = Vec::with_capacity(HEADER_LEN + body.len());
            out.extend_from_slice(&checked[..SHORT_HEADER_LEN]);
            out.extend_from_slice(&crc32(body).to_le_bytes());
            out.resize(HEADER_LEN, 0);
            out.extend_from_slice(body);
            Ok(out)
        }
        other => Err(ParseError::UnknownHeaderType(other)),
    }
}

/// The inverse of [`widen`]: one format writer's type-1 image as the file to emit.
///
/// Anything that is not type 0 passes through, including a generation this build has no
/// layout for — a writer emits what it was given, and only [`widen`] has to understand
/// the bytes well enough to refuse them.
///
/// ⚠️ The 16 unclaimed bytes at `0x1c` have no home in a type-0 file and are dropped.
/// That is lossless for anything that came through [`widen`], which synthesised them as
/// zeros, and it is the only way they are ever reached.
pub fn narrow(image: &[u8]) -> Vec<u8> {
    if !matches!(header_type(image), Ok(TYPE_SHORT)) || image.len() < HEADER_LEN {
        return image.to_vec();
    }
    let mut out = Vec::with_capacity(image.len() - SIZE_DELTA);
    out.extend_from_slice(&image[..SHORT_HEADER_LEN]);
    out.extend_from_slice(&image[HEADER_LEN..]);
    let sum = crc16(&out);
    out.extend_from_slice(&sum.to_le_bytes());
    out
}

/// Read one fixed-length entity of either generation, as its type-1 image.
///
/// Consumes exactly the bytes the file occupies in its own generation, so the reader is
/// left where the next entity would start.
pub(crate) fn read_fixed(
    reader: &mut (impl Read + Seek),
    type1_len: usize,
) -> Result<Vec<u8>, Error> {
    let start = reader.stream_position()?;
    let mut head = [0u8; TYPE_AT + 4];
    reader.read_exact(&mut head)?;
    reader.seek(SeekFrom::Start(start))?;

    let mut bytes = vec![0u8; stored_len(header_type(&head)?, type1_len)];
    reader.read_exact(&mut bytes)?;
    Ok(widen(&bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A type-1 file is not touched, whatever it holds.
    #[test]
    fn a_type_1_file_passes_through_both_ways() {
        let mut file = b"CBIN\x01\x00\x00\x00ne5p".to_vec();
        file.resize(HEADER_LEN + 8, 0xab);
        assert_eq!(widen(&file).unwrap(), file);
        assert_eq!(narrow(&file), file);
    }

    /// Widening and narrowing are inverses, which is what makes a byte-exact round trip
    /// of a type-0 file possible at all.
    #[test]
    fn a_type_0_file_survives_a_widen_and_a_narrow() {
        let mut file = b"CBIN\x00\x00\x00\x00ne5p".to_vec();
        file.resize(SHORT_HEADER_LEN, 0);
        file.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        file.extend_from_slice(&crc16(&file).to_le_bytes());

        let image = widen(&file).unwrap();
        assert_eq!(image.len(), file.len() + SIZE_DELTA);
        // The unclaimed bytes are synthesised, so they had better be the zeros `narrow`
        // is entitled to drop.
        assert_eq!(&image[0x1c..HEADER_LEN], &[0u8; 16]);
        assert_eq!(narrow(&image), file);
    }

    /// A corrupted type-0 file is refused. The format's own crc32 assertion cannot
    /// catch it — the crc32 is synthesised from whatever body arrived.
    #[test]
    fn a_bad_type_0_checksum_is_refused() {
        let mut file = b"CBIN\x00\x00\x00\x00ne5p".to_vec();
        file.resize(SHORT_HEADER_LEN, 0);
        file.extend_from_slice(&[1, 2, 3, 4]);
        file.extend_from_slice(&crc16(&file).to_le_bytes());

        let len = file.len();
        file[len - 3] ^= 0xff;
        assert!(widen(&file).is_err());
    }

    /// A generation this build has no layout for is refused rather than decoded on a
    /// guess about where its body starts.
    #[test]
    fn an_unknown_header_type_is_refused() {
        let mut file = b"CBIN\x02\x00\x00\x00ne5p".to_vec();
        file.resize(HEADER_LEN + 8, 0);
        assert!(matches!(
            widen(&file),
            Err(ParseError::UnknownHeaderType(2))
        ));
    }
}
