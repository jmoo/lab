//! Converting between the wire's entity **body** and an on-disk `CBIN` file.
//!
//! The device transfers only the body — 121 bytes for an Electro 5 program — while a
//! `.ne5p` on disk is that body behind a `CBIN` header. The header is fully determined
//! by the body plus the slot it came from, so a read can be turned into a byte-exact
//! file and a file can be stripped back down for a write. The header codec and the
//! checksum live in `nord_format::cbin`; this module only pairs a wire body with the
//! header the device implies.
//!
//! **Verified**: rebuilding the header from `(body, bank, slot, version)` reproduces all
//! 41 program specimens in the corpus byte-for-byte.
//!
//! ⚠️ The header is *not* fully determined by the body and slot alone: the schema version
//! at `0x14` differs per format tag (`ne5p` is 4, `ne5t` is 0 or 1), so it has to be
//! supplied by the caller from the device's own `0x1e` object-info response. Substituting
//! a constant reproduces programs correctly and silently corrupts every other class.

use crate::error::{Error, Result};
use crate::wire::Location;
use nord_format::cbin::{self, Cbin, Header, RawBody};
use std::io::Cursor;

/// CRC-32/ISO-HDLC over a wire body — the same checksum the type-1 container
/// carries, for comparing against the device's own `0x1e` report.
pub fn crc32(data: &[u8]) -> u32 {
    nord_format::crc::crc32(data)
}

/// Wrap a wire body in a `CBIN` header, producing the bytes of a `.ne5p`-style file.
///
/// `format` and `version` are the tag and schema version the device reported for the
/// slot — both come from `0x1e` object info. `version` is per format tag, so passing a
/// program's 4 for a set list writes a header `nord-format` will refuse to read.
pub fn wrap(format: &str, at: Location, version: u32, body: &[u8]) -> Result<Vec<u8>> {
    if format.len() != 4 {
        return Err(Error::Envelope(format!(
            "format tag {format:?} is not 4 characters"
        )));
    }

    // Zero-indexed, the same numbering the wire uses — one below the display.
    let file = Cbin {
        header: Header::new(format, (at.bank as u16, at.slot as u16), version),
        body: RawBody(body.to_vec()),
    };
    let mut out = Cursor::new(Vec::new());
    file.write_to(&mut out)
        .map_err(|e| Error::Envelope(e.to_string()))?;
    Ok(out.into_inner())
}

/// The inverse: take file bytes and hand back the body the wire wants, plus the
/// format tag and the slot the file claims to belong to. The container checksum is
/// verified on the way.
pub fn unwrap(file: &[u8]) -> Result<(String, Location, Vec<u8>)> {
    let read =
        cbin::read_raw(&mut Cursor::new(file)).map_err(|e| Error::Envelope(e.to_string()))?;
    let (bank, slot) = read.header.slot();
    Ok((
        String::from_utf8_lossy(&read.header.tag).into_owned(),
        Location {
            bank: bank as u32,
            slot: slot as u32,
        },
        read.body.0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real `.ne5p` read off bank 8 slot 14, split at the header boundary.
    const HEADER: &str =
        "4342494e010000006e65357007000d00ffffffff04000000b65d46a500000000000000000000000000000000";
    const BODY: &str = "000401df06781fc60000000000000000000000000000000000000100000000000000000000400000000000000002200000000000022000400000008888000008008888000008000000000080000000000080000000000000000000800000000800800000000800020010060401020408140010000000000000";

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn wrap_rebuilds_the_original_file() {
        let body = hex(BODY);
        let file = [hex(HEADER), body.clone()].concat();
        // Bank 8 slot 14 on the instrument; 7 and 13 on the wire.
        let built = wrap("ne5p", Location::from_user(8, 14), 4, &body).unwrap();
        assert_eq!(built, file, "rebuilt header differs from the real file");
    }

    /// The version is the device's to report, not ours to assume: a set list is 0 or 1
    /// where a program is 4, and stamping a constant makes `nord-format` refuse the file.
    #[test]
    fn wrap_writes_the_version_it_is_given() {
        let body = hex(BODY);
        for version in [0u32, 1, 4, 540] {
            let file = wrap("ne5t", Location::from_user(1, 1), version, &body).unwrap();
            assert_eq!(
                u32::from_le_bytes(file[0x14..0x18].try_into().unwrap()),
                version
            );
        }
    }

    #[test]
    fn unwrap_is_the_inverse() {
        let body = hex(BODY);
        let file = wrap("ne5p", Location::from_user(8, 14), 4, &body).unwrap();
        let (format, at, got) = unwrap(&file).unwrap();
        assert_eq!(format, "ne5p");
        assert_eq!(at, Location::from_user(8, 14));
        assert_eq!(got, body);
    }

    #[test]
    fn unwrap_rejects_a_corrupted_body() {
        let body = hex(BODY);
        let mut file = wrap("ne5p", Location::from_user(8, 14), 4, &body).unwrap();
        *file.last_mut().unwrap() ^= 0xFF;
        assert!(
            unwrap(&file).is_err(),
            "a corrupted body should fail the checksum"
        );
    }
}
