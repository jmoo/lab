//! Checksums over a region of a stream, at both widths a CBIN container uses.
//!
//! A type-1 container checksums its body with a CRC-32 stored *ahead* of it; a type-0
//! container checksums the whole file with a CRC-16 stored *after* it. [`Width`] is the
//! only thing that differs — the region arithmetic and the writer's buffering are
//! written once and shared.

use crcxx::crc32::{catalog::CRC_32_ISO_HDLC, *};

use crate::error::Error;
use std::fmt::Debug;
use std::io;
use std::io::{Seek, SeekFrom, Write};

const CRC_32_SLICES: usize = 16;
const CRC_32: Crc<LookupTable256xN<CRC_32_SLICES>> =
    Crc::<LookupTable256xN<CRC_32_SLICES>>::new(&CRC_32_ISO_HDLC);

const CRC_16: crcxx::crc16::Crc<crcxx::crc16::LookupTable256> =
    crcxx::crc16::Crc::<crcxx::crc16::LookupTable256>::new(&crcxx::crc16::catalog::CRC_16_IBM_3740);

/// CRC-32 (ISO-HDLC) of a contiguous slice.
pub fn crc32(bytes: &[u8]) -> u32 {
    CRC_32.compute(bytes)
}

/// CRC-16/IBM-3740 — the CCITT-FALSE parameters — of a contiguous slice.
pub fn crc16(bytes: &[u8]) -> u16 {
    CRC_16.compute(bytes)
}

/// One checksum width, as the streaming machinery needs it.
///
/// Implemented for [`Crc32`] and [`Crc16`], which wrap nothing but their accumulator:
/// everything about *where* the checksum applies lives in [`MultipartCrc`].
pub trait Width: Sized {
    /// The checksum as a container stores it.
    type Value: Copy + PartialEq + Debug;

    /// Bytes the stored checksum occupies.
    const LEN: usize;

    /// What [`CrcWriter`] emits in the checksum's place while the region it covers is
    /// still being written, and overwrites once that region completes.
    const PLACEHOLDER: Self::Value;

    fn start() -> Self;
    fn update(&mut self, bytes: &[u8]);
    fn value(&self) -> Self::Value;

    /// Emit the checksum the way a container stores it: little-endian, like every other
    /// field of a CBIN header.
    fn write_le(value: Self::Value, out: &mut impl Write) -> io::Result<()>;
}

/// CRC-32/ISO-HDLC — the type-1 container's checksum over its body.
pub struct Crc32(ComputeMultipart<'static, LookupTable256xN<CRC_32_SLICES>>);

impl Width for Crc32 {
    type Value = u32;
    const LEN: usize = 4;
    const PLACEHOLDER: u32 = 0xFFFFFFFF;

    fn start() -> Crc32 {
        Crc32(CRC_32.compute_multipart())
    }

    fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    fn value(&self) -> u32 {
        self.0.value()
    }

    fn write_le(value: u32, out: &mut impl Write) -> io::Result<()> {
        out.write_all(&value.to_le_bytes())
    }
}

/// CRC-16/IBM-3740 — the type-0 container's checksum over the whole file ahead of it.
pub struct Crc16(crcxx::crc16::ComputeMultipart<'static, crcxx::crc16::LookupTable256>);

impl Width for Crc16 {
    type Value = u16;
    const LEN: usize = 2;
    const PLACEHOLDER: u16 = 0xFFFF;

    fn start() -> Crc16 {
        Crc16(CRC_16.compute_multipart())
    }

    fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    fn value(&self) -> u16 {
        self.0.value()
    }

    fn write_le(value: u16, out: &mut impl Write) -> io::Result<()> {
        out.write_all(&value.to_le_bytes())
    }
}

/// Accumulates the checksum of bytes falling inside one region of a stream.
///
/// ⚠️ `length` is the region's **last byte index minus `first_byte`** — an inclusive
/// span, so the region covers `length + 1` bytes. Passing an end-exclusive bound instead
/// reads one stray byte and, worse, stops [`CrcWriter`] from ever flushing a region that
/// ends at EOF.
pub struct MultipartCrc<W: Width> {
    accumulator: W,
    pub first_byte: u64,
    pub length: u64,
}

/// The type-1 container's width, over a region.
pub type MultipartCrc32 = MultipartCrc<Crc32>;

/// The type-0 container's width, over a region.
pub type MultipartCrc16 = MultipartCrc<Crc16>;

impl<W: Width> MultipartCrc<W> {
    pub fn new(first_byte: u64, length: u64) -> MultipartCrc<W> {
        MultipartCrc {
            accumulator: W::start(),
            first_byte,
            length,
        }
    }

    pub fn update(&mut self, pos: u64, bytes: &[u8]) {
        // The bytes are contiguous from `pos`, so the part inside the region is a
        // single slice: the overlap of `pos..pos + len` with the inclusive span.
        let end = pos + bytes.len() as u64;
        let lo = self.first_byte.max(pos);
        let hi = (self.first_byte + self.length + 1).min(end);
        if lo < hi {
            self.accumulator
                .update(&bytes[(lo - pos) as usize..(hi - pos) as usize]);
        }
    }

    pub fn checksum(&self) -> W::Value {
        self.accumulator.value()
    }
}

/// Passes writes through, checksumming one region of them.
///
/// A checksum stored *after* its region — the type-0 trailing CRC-16 — needs nothing
/// from this beyond the accumulation: by the time [`CrcWriter::checksum`] is asked, the
/// region is already behind the write position and the answer is final.
///
/// A checksum stored *ahead* of its region — the type-1 CRC-32 at `0x18` — cannot be.
/// `checksum()` answers [`Width::PLACEHOLDER`], swallows the placeholder bytes the
/// caller then writes, and buffers everything after them until the region completes; at
/// that moment the real value goes out in the placeholder's place and the buffer follows.
pub struct CrcWriter<'a, S: Write + Seek, W: Width> {
    calc: MultipartCrc<W>,
    inner: &'a mut S,
    buffer: Vec<u8>,
    buffer_pos: u64,
    buffer_writes: bool,
    skip_bytes: u64,
}

impl<'a, S: Write + Seek, W: Width> CrcWriter<'a, S, W> {
    pub fn new(inner: &'a mut S, first_byte: u64, length: u64) -> CrcWriter<'a, S, W> {
        CrcWriter {
            calc: MultipartCrc::new(first_byte, length),
            buffer: Vec::new(),
            inner,
            buffer_writes: false,
            buffer_pos: 0,
            skip_bytes: 0,
        }
    }

    /// The checksum, or a placeholder plus the buffering that makes one possible.
    pub fn checksum(&mut self) -> Result<W::Value, Error> {
        let pos = self.inner.stream_position()?;

        // Past the region: the answer is already final.
        if pos > self.calc.first_byte && pos > (self.calc.first_byte + self.calc.length) {
            return Ok(self.calc.checksum());
        }

        // Ahead of the region: buffer until it completes, then patch the placeholder.
        if pos < self.calc.first_byte {
            if self.buffer_writes {
                return Err(Error::Io(io::Error::other(
                    "Attempted to calculate multiple checksums with single instance",
                )));
            }

            self.buffer_writes = true;
            self.buffer_pos = pos;
            self.skip_bytes = W::LEN as u64;
            return Ok(W::PLACEHOLDER);
        }

        Err(Error::Io(io::Error::other(
            "Attempted to calculate checksum in the middle of the buffer",
        )))
    }
}

impl<S: Write + Seek, W: Width> Write for CrcWriter<'_, S, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.buffer_writes {
            let mut offset: u64 = 0;

            for byte in buf {
                if self.skip_bytes > 0 {
                    offset += 1;
                    self.skip_bytes -= 1;
                    continue;
                }

                self.buffer.push(*byte);
                offset += 1;
            }

            self.calc.update(self.buffer_pos, buf);
            self.buffer_pos += offset;

            if self.buffer_pos > (self.calc.first_byte + self.calc.length) {
                self.buffer_writes = false;

                W::write_le(self.calc.checksum(), &mut self.inner)?;
            } else {
                return Ok(buf.len());
            }
        }

        if !self.buffer.is_empty() {
            // `write_all`, not `write`: a short write here would silently drop the tail
            // of the buffered body while this call reports full success.
            self.inner.write_all(self.buffer.as_slice())?;
            self.buffer.clear();
            return Ok(buf.len());
        }

        let pos = self.inner.stream_position()?;

        // Only the bytes accepted — hashing past `size` would checksum bytes the
        // caller will retry, counting them twice.
        match self.inner.write(buf) {
            Ok(size) => {
                self.calc.update(pos, &buf[..size]);
                Ok(size)
            }
            Err(e) => Err(e),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl<S: Write + Seek, W: Width> Seek for CrcWriter<'_, S, W> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// ⚠️ The second argument to `new` is the *last byte index* of the region minus the
    /// first — an inclusive span, not a byte count. Passing the end-exclusive form stops
    /// the writer from ever flushing.
    ///
    /// The layout mirrors a type-1 container: the checksum field precedes the body, so
    /// `checksum()` is called ahead of the region and writes buffer until the region
    /// completes. The region here ends at the last byte of the stream — nothing follows
    /// to push the position past it — so the flush must fire when the region is
    /// complete, not only once something is written beyond it.
    #[test]
    fn a_region_ending_at_eof_still_flushes() {
        // 8 bytes ahead, crc at 8..12, body at 12..=15 — the file ends with the region.
        let (first, last) = (12u64, 15u64);
        let mut inner = Cursor::new(Vec::new());
        {
            let mut w = CrcWriter::<_, Crc32>::new(&mut inner, first, last - first);
            w.write_all(&[0u8; 8]).unwrap();
            let placeholder = w.checksum().unwrap();
            assert_eq!(placeholder, 0xFFFFFFFF, "not yet computable: a placeholder");
            Crc32::write_le(placeholder, &mut w).unwrap();
            w.write_all(&[0xaa, 0xbb, 0xcc, 0xdd]).unwrap();
        }

        let out = inner.into_inner();
        assert_eq!(out.len(), 16, "the buffered body was never flushed");
        assert_eq!(&out[12..], &[0xaa, 0xbb, 0xcc, 0xdd]);

        // The checksum written at 8..12 is the checksum of the region.
        let mut crc = MultipartCrc32::new(first, last - first);
        crc.update(0, &out);
        assert_eq!(&out[8..12], &crc.checksum().to_le_bytes());
    }

    /// A checksum that sits after its region needs none of the buffering: it is asked
    /// for once the region is behind the write position, and answers straight away.
    #[test]
    fn a_trailing_checksum_never_buffers() {
        let mut inner = Cursor::new(Vec::new());
        {
            let mut w = CrcWriter::<_, Crc16>::new(&mut inner, 0, 7);
            w.write_all(&[0x11, 0x22, 0x33, 0x44]).unwrap();
            w.write_all(&[0x55, 0x66, 0x77, 0x88]).unwrap();
            let sum = w.checksum().unwrap();
            assert_eq!(
                sum,
                crc16(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88])
            );
            Crc16::write_le(sum, &mut w).unwrap();
        }

        let out = inner.into_inner();
        assert_eq!(out.len(), 10);
        assert_eq!(&out[8..], &crc16(&out[..8]).to_le_bytes());
    }

    /// The region arithmetic is one implementation, so both widths agree on which bytes
    /// a region covers.
    #[test]
    fn both_widths_measure_the_same_region() {
        let data: Vec<u8> = (0u8..32).collect();

        let mut wide = MultipartCrc32::new(8, 15);
        wide.update(0, &data);
        assert_eq!(wide.checksum(), crc32(&data[8..24]));

        let mut narrow = MultipartCrc16::new(8, 15);
        narrow.update(0, &data);
        assert_eq!(narrow.checksum(), crc16(&data[8..24]));
    }
}
