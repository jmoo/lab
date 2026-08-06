//! The typed layer over the CBIN container: a [`File`] is one format's decoded body
//! inside the header it arrived with.
//!
//! Reading is streaming: the fixed-size header prefix is parsed, the format's identity
//! is gated (tag, version, trailer, location), and then the body is handed to the
//! format through a [`BodyReader`] that checksums the bytes as they pass. The stored
//! checksum is settled in the same pass — a corrupt file fails before its decoded value
//! escapes, and no body is buffered that the format did not ask for.
//!
//! What lives where:
//!
//! * **Per container generation** — header length, checksum kind and placement — stays
//!   in [`container`] and this module's read/write plumbing. A format never sees a CRC.
//! * **Per format** — tag, versions, body codec, location reading — is a [`Format`]
//!   impl, one per format module.

use std::io;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use crate::common::container::{self, Container, Header};
use crate::crc::{Crc16, Crc32, Width};
use crate::error::{Error, ParseError};
use crate::types::RangedU16Pair;

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// How a format reads the `0x0c` location field. The wire value is always carried;
/// the impl decides what it means.
pub trait Location: Copy {
    fn from_wire(location: u32) -> Result<Self, ParseError>;
    fn to_wire(&self) -> u32;
}

/// A slotted format's bank and slot, bounds-checked into the format's own space.
impl<const X: u16, const Y: u16> Location for RangedU16Pair<X, Y> {
    fn from_wire(location: u32) -> Result<Self, ParseError> {
        (location as u16, (location >> 16) as u16).try_into()
    }

    fn to_wire(&self) -> u32 {
        container::location_of(self.x(), self.y())
    }
}

/// The location field held verbatim, for formats that address no slot: a library
/// sample carries `0xFFFFFFFF` (no slot until an instrument gives it one), and a
/// piano library's field is unmapped. Preserved rather than assumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verbatim(pub u32);

impl Location for Verbatim {
    fn from_wire(location: u32) -> Result<Verbatim, ParseError> {
        Ok(Verbatim(location))
    }

    fn to_wire(&self) -> u32 {
        self.0
    }
}

/// An unmapped body, held verbatim: enough to identify a file and hand it back
/// unchanged, and no more.
#[derive(Clone, PartialEq, Eq)]
pub struct Opaque(pub Vec<u8>);

impl std::fmt::Debug for Opaque {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Opaque({} bytes)", self.0.len())
    }
}

/// One CBIN format: its tag, its versions, and how to interpret what the container
/// hands over. Implemented by each format module's zero-sized marker; sealed so the
/// codec machinery (`binrw` included) stays inside the crate.
pub trait Format: sealed::Sealed + Sized + std::fmt::Debug {
    const TAG: &'static str;

    /// Body-schema versions this build's field offsets are validated against.
    /// Empty means ungated: the version field is content, not a schema revision
    /// (a sample stores `format * 100 + revision` there).
    const KNOWN_VERSIONS: &'static [u32];

    /// Total file length **as type 1** for a fixed-length format; `None` for a format
    /// whose body runs to the end of the file. The body is the same length in both
    /// generations, so one number covers both.
    const FILE_LEN: Option<usize>;

    type Location: Location + std::fmt::Debug;
    type Body: std::fmt::Debug;

    /// Decode the body. `r` is positioned at the body's first byte, capped at the
    /// body's extent, and already inside the container's checksum accounting; the impl
    /// must consume the body exactly — [`File::read_from`] refuses a short read.
    fn read_body(r: &mut BodyReader, header: &Header) -> Result<Self::Body, Error>;

    /// Encode the body. `header` is the header the file will carry — a set list
    /// echoes its version into the body, which is why it is passed.
    fn write_body(
        body: &Self::Body,
        header: &Header,
        w: &mut (impl Write + Seek),
    ) -> Result<(), Error>;

    /// Header checks beyond tag and version. The default is the trailer every format
    /// addressed by slot carries; a library format overrides it (a sample's trailer is
    /// `0x000f0000` on every specimen, meaning unknown, preserved verbatim).
    fn check(header: &Header) -> Result<(), ParseError> {
        if header.trailer != container::SLOT_TRAILER {
            return Err(ParseError::AssertFail(format!(
                "{}: header trailer is {:#010x}, expected {:#010x}",
                Self::TAG,
                header.trailer,
                container::SLOT_TRAILER
            )));
        }
        Ok(())
    }
}

/// The checksum a body's bytes are fed through on the way in — whichever one the
/// file's generation carries.
enum Accum {
    /// Type 1: crc32 over the body alone.
    Body(Crc32),
    /// Type 0: crc16 over the whole file, primed with the header before the body
    /// starts flowing.
    WholeFile(Crc16),
    /// A wire body: no container, nothing to verify.
    Untracked,
}

impl Accum {
    fn update(&mut self, bytes: &[u8]) {
        match self {
            Accum::Body(c) => c.update(bytes),
            Accum::WholeFile(c) => c.update(bytes),
            Accum::Untracked => {}
        }
    }
}

/// The body's bytes as [`File::read_from`] hands them to a format: capped at the
/// body's extent, checksummed as they pass.
///
/// ⚠️ Reads are fed to the checksum in order, so the body must be consumed
/// sequentially, each byte once. A format that buffers via [`BodyReader::bytes`] and
/// decodes the buffer cannot get this wrong.
pub struct BodyReader<'a> {
    inner: &'a mut dyn Read,
    accum: Accum,
    remaining: u64,
}

impl BodyReader<'_> {
    /// Bytes of body not yet consumed.
    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    /// The whole remaining body, buffered — the right call for a body that is small
    /// or that `binrw` will decode from a slice. A format streaming a large body reads
    /// incrementally instead.
    pub fn bytes(&mut self) -> Result<Vec<u8>, Error> {
        let expected = self.remaining;
        let mut out = Vec::with_capacity(expected as usize);
        self.read_to_end(&mut out)?;
        if self.remaining != 0 {
            return Err(ParseError::AssertFail(format!(
                "body: expected {expected} bytes, the stream ended {} short",
                self.remaining
            ))
            .into());
        }
        Ok(out)
    }
}

impl Read for BodyReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let cap = self.remaining.min(buf.len() as u64) as usize;
        if cap == 0 {
            return Ok(0);
        }
        let n = self.inner.read(&mut buf[..cap])?;
        self.accum.update(&buf[..n]);
        self.remaining -= n as u64;
        Ok(n)
    }
}

/// One file of format `F`: the header it arrived with (or will be written with), its
/// location in `F`'s own space, and the decoded body.
///
/// The header is carried verbatim so the file goes back out as the one that came in —
/// its generation, its version, and the 16 bytes no format claims. See
/// [`Header::header_type`] for why the generation must survive.
#[derive(Debug)]
pub struct File<F: Format> {
    pub header: Header,
    pub location: F::Location,
    pub body: F::Body,
}

impl<F: Format> File<F> {
    /// Read one file of either generation, verifying its checksum in the same pass
    /// that decodes it.
    ///
    /// Consumes exactly the bytes the file occupies in its own generation, so the
    /// reader is left where the next entity would start.
    pub fn read_from(reader: &mut (impl Read + Seek)) -> Result<File<F>, Error> {
        let mut head = [0u8; container::SHORT_HEADER_LEN];
        reader.read_exact(&mut head)?;
        let header_type = container::header_type(&head)?;
        if header_type != container::TYPE_LONG && header_type != container::TYPE_SHORT {
            return Err(ParseError::UnknownHeaderType(header_type).into());
        }

        let le = |at: usize| u32::from_le_bytes(head[at..at + 4].try_into().unwrap());
        let mut header = Header {
            header_type,
            tag: String::from_utf8_lossy(&head[0x08..0x0c]).into_owned(),
            trailer: le(0x10),
            version: le(0x14),
            unclaimed: [0; 16],
        };

        // The identity gates, in the order the errors are most useful: what the file
        // is, whether this build reads that revision, then the header oddities.
        if header.tag != F::TAG {
            return Err(ParseError::WrongFormat {
                expected: F::TAG,
                got: header.tag,
            }
            .into());
        }
        if !F::KNOWN_VERSIONS.is_empty() && !F::KNOWN_VERSIONS.contains(&header.version) {
            return Err(ParseError::UnsupportedVersion {
                format: F::TAG,
                version: header.version,
                supported: F::KNOWN_VERSIONS,
            }
            .into());
        }
        F::check(&header)?;
        let location = F::Location::from_wire(le(0x0c))?;

        let stored_crc32 = match header_type {
            container::TYPE_LONG => {
                let mut rest = [0u8; container::HEADER_LEN - container::SHORT_HEADER_LEN];
                reader.read_exact(&mut rest)?;
                header.unclaimed = rest[4..].try_into().unwrap();
                Some(u32::from_le_bytes(rest[..4].try_into().unwrap()))
            }
            _ => None,
        };

        let body_len = match F::FILE_LEN {
            Some(len) => (len - container::HEADER_LEN) as u64,
            // A variable-length body runs to the end of the stream, less the trailing
            // crc16 a type-0 file keeps after it.
            None => {
                let at = reader.stream_position()?;
                let end = reader.seek(SeekFrom::End(0))?;
                reader.seek(SeekFrom::Start(at))?;
                let tail = match header_type {
                    container::TYPE_SHORT => container::CRC16_LEN as u64,
                    _ => 0,
                };
                (end - at).checked_sub(tail).ok_or_else(|| {
                    ParseError::AssertFail(format!(
                        "{}: {} bytes after the header is too short to hold the \
                         trailing checksum",
                        F::TAG,
                        end - at
                    ))
                })?
            }
        };

        let accum = match header_type {
            container::TYPE_SHORT => {
                let mut crc = Crc16::start();
                crc.update(&head);
                Accum::WholeFile(crc)
            }
            _ => Accum::Body(Crc32::start()),
        };
        let mut body_reader = BodyReader {
            inner: reader,
            accum,
            remaining: body_len,
        };
        let body = F::read_body(&mut body_reader, &header)?;
        if body_reader.remaining != 0 {
            return Err(ParseError::AssertFail(format!(
                "{}: the body was consumed {} bytes short of its end",
                F::TAG,
                body_reader.remaining
            ))
            .into());
        }

        match (body_reader.accum, stored_crc32) {
            (Accum::Body(crc), Some(stored)) => {
                let computed = crc.value();
                if computed != stored {
                    return Err(ParseError::AssertFail(format!(
                        "type-1 CBIN: stored checksum {stored:#010x} does not match the \
                         body's {computed:#010x}"
                    ))
                    .into());
                }
            }
            (Accum::WholeFile(crc), None) => {
                let mut sum = [0u8; container::CRC16_LEN];
                reader.read_exact(&mut sum)?;
                let stored = u16::from_le_bytes(sum);
                let computed = crc.value();
                if computed != stored {
                    return Err(ParseError::AssertFail(format!(
                        "type-0 CBIN: stored checksum {stored:#06x} does not match the \
                         file's {computed:#06x}"
                    ))
                    .into());
                }
            }
            _ => unreachable!("the accumulator is chosen by the same header type"),
        }

        Ok(File {
            header,
            location,
            body,
        })
    }

    /// Emit the file, in its own generation, with the checksum recomputed over the
    /// bytes actually written.
    pub fn write_to(&self, writer: &mut (impl Write + Seek)) -> Result<(), Error> {
        self.container()?.write_to(writer)
    }

    /// The file's bytes — [`File::write_to`] into memory.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        self.container()?.to_bytes()
    }

    /// The body alone — what the wire transfers.
    pub fn body_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(self.container()?.body)
    }

    /// A wire body under a fresh type-1 header — the counterpart to
    /// [`File::body_bytes`], for the device path where only the body ever crosses.
    ///
    /// `version` is the device's to report, per format tag, never a constant: see
    /// `nord-usb`'s envelope notes.
    pub fn from_wire(location: F::Location, version: u32, body: &[u8]) -> Result<File<F>, Error> {
        let header = Header::new(F::TAG, version);
        if !F::KNOWN_VERSIONS.is_empty() && !F::KNOWN_VERSIONS.contains(&version) {
            return Err(ParseError::UnsupportedVersion {
                format: F::TAG,
                version,
                supported: F::KNOWN_VERSIONS,
            }
            .into());
        }
        let mut cursor = Cursor::new(body);
        let mut reader = BodyReader {
            inner: &mut cursor,
            accum: Accum::Untracked,
            remaining: body.len() as u64,
        };
        let decoded = F::read_body(&mut reader, &header)?;
        if reader.remaining != 0 {
            return Err(ParseError::AssertFail(format!(
                "{}: a {}-byte wire body left {} bytes unread",
                F::TAG,
                body.len(),
                reader.remaining
            ))
            .into());
        }
        Ok(File {
            header,
            location,
            body: decoded,
        })
    }

    /// The container this file emits as: the carried header, the location back on the
    /// wire, and the body re-encoded — with a fixed-length format's emitted length
    /// checked against what its module declares, so a wrong writer is caught here
    /// rather than producing a file that looks plausible until something loads it.
    fn container(&self) -> Result<Container, Error> {
        let mut out = Cursor::new(Vec::new());
        F::write_body(&self.body, &self.header, &mut out)?;
        let body = out.into_inner();

        if let Some(file_len) = F::FILE_LEN {
            if body.len() != file_len - container::HEADER_LEN {
                // Reported as whole-file lengths in the file's own generation, the
                // numbers a reader can check against the file on disk.
                return Err(ParseError::BadEncodedLength {
                    format: F::TAG,
                    got: container::stored_len(
                        self.header.header_type,
                        body.len() + container::HEADER_LEN,
                    ),
                    expected: container::stored_len(self.header.header_type, file_len),
                }
                .into());
            }
        }

        Ok(Container {
            header: self.header.clone(),
            location: self.location.to_wire(),
            body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use binrw::{BinRead, BinWriterExt};

    /// A minimal fixed-length format: tag `test`, version 4, an 8-byte body of two
    /// big-endian u32s, slotted 0..=8 / 0..=50.
    #[derive(Debug)]
    struct TestFormat;

    #[derive(Debug, BinRead, binrw::BinWrite, PartialEq)]
    #[brw(big)]
    struct TestBody {
        a: u32,
        b: u32,
    }

    impl sealed::Sealed for TestFormat {}
    impl Format for TestFormat {
        const TAG: &'static str = "test";
        const KNOWN_VERSIONS: &'static [u32] = &[4];
        const FILE_LEN: Option<usize> = Some(container::HEADER_LEN + 8);
        type Location = RangedU16Pair<8, 50>;
        type Body = TestBody;

        fn read_body(r: &mut BodyReader, _header: &Header) -> Result<TestBody, Error> {
            Ok(TestBody::read(&mut Cursor::new(r.bytes()?))?)
        }

        fn write_body(
            body: &TestBody,
            _header: &Header,
            w: &mut (impl Write + Seek),
        ) -> Result<(), Error> {
            w.write_be(body)?;
            Ok(())
        }
    }

    fn specimen(header_type: u32) -> Vec<u8> {
        let mut file = File::<TestFormat> {
            header: Header::new("test", 4),
            location: (3, 7).try_into().unwrap(),
            body: TestBody { a: 1, b: 2 },
        };
        file.header.header_type = header_type;
        file.header.unclaimed = match header_type {
            container::TYPE_LONG => [0xab; 16],
            _ => [0; 16],
        };
        file.to_bytes().unwrap()
    }

    /// Both generations stream in, verify, and go back out as the file they came from.
    #[test]
    fn either_generation_round_trips_byte_for_byte() {
        for header_type in [container::TYPE_LONG, container::TYPE_SHORT] {
            let bytes = specimen(header_type);
            let mut cursor = Cursor::new(&bytes);
            let file = File::<TestFormat>::read_from(&mut cursor).unwrap();
            assert_eq!(file.header.header_type, header_type);
            assert_eq!(file.location.inner(), (3, 7));
            assert_eq!(file.body, TestBody { a: 1, b: 2 });
            assert_eq!(file.to_bytes().unwrap(), bytes);
            // The whole extent was consumed: the reader sits at the file's end.
            assert_eq!(cursor.position() as usize, bytes.len());
        }
    }

    /// Each generation is checked against its own checksum in the decode pass itself.
    #[test]
    fn a_corrupted_body_is_refused_in_both_generations() {
        for header_type in [container::TYPE_LONG, container::TYPE_SHORT] {
            let mut bytes = specimen(header_type);
            let at = container::body_at(header_type);
            bytes[at] ^= 0xff;
            let err = File::<TestFormat>::read_from(&mut Cursor::new(&bytes))
                .expect_err("a corrupted body must not decode");
            assert!(err.to_string().contains("checksum"), "{err}");
        }
    }

    /// The identity gates, each with its own refusal — the checks `Container::open`
    /// used to make, now settled before the body is read.
    #[test]
    fn the_gates_refuse_a_tag_a_version_a_trailer_and_a_slot() {
        let read = |bytes: &[u8]| File::<TestFormat>::read_from(&mut Cursor::new(bytes));
        assert!(read(&specimen(container::TYPE_LONG)).is_ok());

        let mut wrong_tag = specimen(container::TYPE_LONG);
        wrong_tag[0x08..0x0c].copy_from_slice(b"ne5l");
        assert!(matches!(
            read(&wrong_tag),
            Err(Error::Parse(ParseError::WrongFormat {
                expected: "test",
                ..
            }))
        ));

        let bad = |patch: fn(&mut File<TestFormat>)| {
            let mut file = File::<TestFormat> {
                header: Header::new("test", 4),
                location: (3, 7).try_into().unwrap(),
                body: TestBody { a: 1, b: 2 },
            };
            patch(&mut file);
            file.to_bytes().unwrap()
        };

        assert!(matches!(
            read(&bad(|f| f.header.version = 5)),
            Err(Error::Parse(ParseError::UnsupportedVersion {
                version: 5,
                ..
            }))
        ));
        assert!(matches!(
            read(&bad(|f| f.header.trailer = 0)),
            Err(Error::Parse(ParseError::AssertFail(_)))
        ));

        let mut far = specimen(container::TYPE_LONG);
        far[0x0c..0x10].copy_from_slice(&container::location_of(99, 0).to_le_bytes());
        // Corrupting the location invalidates nothing (the type-1 crc32 covers only
        // the body), so the slot bound itself is what refuses.
        assert!(matches!(
            read(&far),
            Err(Error::Parse(ParseError::OutOfBounds { .. }))
        ));
    }

    /// A fixed-length writer that emits the wrong number of bytes is caught at encode.
    #[test]
    fn a_wrong_length_body_is_refused_at_encode() {
        #[derive(Debug)]
        struct Stub;
        impl sealed::Sealed for Stub {}
        impl Format for Stub {
            const TAG: &'static str = "stub";
            const KNOWN_VERSIONS: &'static [u32] = &[];
            const FILE_LEN: Option<usize> = Some(container::HEADER_LEN + 4);
            type Location = Verbatim;
            type Body = Opaque;

            fn read_body(r: &mut BodyReader, _h: &Header) -> Result<Opaque, Error> {
                Ok(Opaque(r.bytes()?))
            }
            fn write_body(
                body: &Opaque,
                _h: &Header,
                w: &mut (impl Write + Seek),
            ) -> Result<(), Error> {
                w.write_all(&body.0)?;
                Ok(())
            }
        }

        let file = File::<Stub> {
            header: Header::new("stub", 0),
            location: Verbatim(0),
            body: Opaque(vec![1, 2, 3]),
        };
        assert!(matches!(
            file.to_bytes(),
            Err(Error::Parse(ParseError::BadEncodedLength {
                format: "stub",
                ..
            }))
        ));
    }

    /// A variable-length body runs to the end of the stream in either generation —
    /// which for type 0 means stopping short of the trailing crc16.
    #[test]
    fn a_variable_body_finds_its_own_extent() {
        #[derive(Debug)]
        struct Var;
        impl sealed::Sealed for Var {}
        impl Format for Var {
            const TAG: &'static str = "vari";
            const KNOWN_VERSIONS: &'static [u32] = &[];
            const FILE_LEN: Option<usize> = None;
            type Location = Verbatim;
            type Body = Opaque;

            fn read_body(r: &mut BodyReader, _h: &Header) -> Result<Opaque, Error> {
                Ok(Opaque(r.bytes()?))
            }
            fn write_body(
                body: &Opaque,
                _h: &Header,
                w: &mut (impl Write + Seek),
            ) -> Result<(), Error> {
                w.write_all(&body.0)?;
                Ok(())
            }
        }

        for header_type in [container::TYPE_LONG, container::TYPE_SHORT] {
            let mut file = File::<Var> {
                header: Header::new("vari", 9),
                location: Verbatim(0xFFFFFFFF),
                body: Opaque(vec![7; 33]),
            };
            file.header.header_type = header_type;
            let bytes = file.to_bytes().unwrap();

            let back = File::<Var>::read_from(&mut Cursor::new(&bytes)).unwrap();
            assert_eq!(back.body.0, vec![7; 33], "type {header_type}");
            assert_eq!(back.to_bytes().unwrap(), bytes);
        }
    }

    /// The wire path: a bare body in, the same bytes back out, no container built in
    /// between.
    #[test]
    fn a_wire_body_round_trips_without_a_container() {
        let body = [0, 0, 0, 1, 0, 0, 0, 2];
        let file = File::<TestFormat>::from_wire((3, 7).try_into().unwrap(), 4, &body).unwrap();
        assert_eq!(file.body, TestBody { a: 1, b: 2 });
        assert_eq!(file.body_bytes().unwrap(), body);

        // And the version gate holds on the wire too.
        assert!(matches!(
            File::<TestFormat>::from_wire((3, 7).try_into().unwrap(), 9, &body),
            Err(Error::Parse(ParseError::UnsupportedVersion {
                version: 9,
                ..
            }))
        ));
    }
}
