//! Converting between the wire's entity **body** and an on-disk `CBIN` file.
//!
//! The device transfers only the body — 121 bytes for an Electro 5 program — while a
//! `.ne5p` on disk is that body inside a `CBIN` container. The container is fully
//! determined by the body plus the slot it came from, so a read can be turned into a
//! byte-exact file and a file can be stripped back down for a write.
//!
//! **Verified**: rebuilding the header from `(body, bank, slot, version)` reproduces all
//! 41 program specimens in the corpus byte-for-byte.
//!
//! ⚠️ The header is *not* fully determined by the body and slot alone: the schema version
//! at `0x14` differs per format tag (`ne5p` is 4, `ne5t` is 0 or 1), so it has to be
//! supplied by the caller from the device's own `0x1e` object-info response. Substituting
//! a constant reproduces programs correctly and silently corrupts every other class.
//!
//! The layout and both checksums are [`nord_format::common::container`]'s; this is the
//! wire's vocabulary over it.

use nord_format::common::container::{self, Container};

use crate::error::{Error, Result};
use crate::wire::Location;

/// Bytes of `CBIN` header ahead of the body, in the generation the device writes.
pub const HEADER_LEN: usize = container::HEADER_LEN;

/// CRC-32/ISO-HDLC, the checksum a `CBIN` container carries over its body.
pub(crate) use nord_format::crc::crc32;

/// Wrap a wire body in a `CBIN` container, producing the bytes of a `.ne5p`-style file.
///
/// `format` and `version` are the tag and schema version the device reported for the
/// slot — both come from `0x1e` object info. `version` is per format tag, so passing a
/// program's 4 for a set list writes a header `nord-format` will refuse to read.
pub fn wrap(format: &str, at: Location, version: u32, body: &[u8]) -> Result<Vec<u8>> {
    // Location is two little-endian u16s, zero-indexed — the same numbering the wire
    // uses, and one below what the instrument displays.
    Container::new(
        format,
        container::location_of(at.bank as u16, at.slot as u16),
        version,
        body.to_vec(),
    )
    .to_bytes()
    .map_err(|e| Error::Envelope(e.to_string()))
}

/// The inverse: take file bytes and hand back the body the wire wants, plus the
/// format tag and the slot the file claims to belong to.
pub fn unwrap(file: &[u8]) -> Result<(String, Location, &[u8])> {
    let container = Container::parse(file).map_err(|e| Error::Envelope(e.to_string()))?;
    if container.body.is_empty() {
        return Err(Error::Truncated {
            got: file.len(),
            need: file.len() + 1,
        });
    }

    let at = Location {
        bank: container.bank() as u32,
        slot: container.slot() as u32,
    };
    // Borrowed from the caller's bytes rather than handed over from the container: the
    // wire sends a body it does not own, and the copy would be pure ceremony.
    let at_body = container::body_at(container.header.header_type);
    Ok((
        container.header.tag,
        at,
        &file[at_body..at_body + container.body.len()],
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
        assert_eq!(got, &body[..]);
    }

    /// A file handed to `put` can be either CBIN generation, so `unwrap` reads the body
    /// from where that generation keeps it rather than from a fixed offset. The device
    /// only ever writes type 1, which is why `wrap` only produces it.
    #[test]
    fn unwrap_reads_a_short_header_file() {
        let body = hex(BODY);
        let mut short =
            Container::parse(&wrap("ne5p", Location::from_user(8, 14), 4, &body).unwrap()).unwrap();
        short.header.header_type = container::TYPE_SHORT;
        let short = short.to_bytes().unwrap();
        assert_eq!(short.len(), HEADER_LEN + body.len() - container::SIZE_DELTA);

        let (format, at, got) = unwrap(&short).unwrap();
        assert_eq!(format, "ne5p");
        assert_eq!(at, Location::from_user(8, 14));
        assert_eq!(got, &body[..]);
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
