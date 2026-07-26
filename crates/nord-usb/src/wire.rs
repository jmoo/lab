//! The vendor wire protocol.
//!
//! Every message on the vendor bulk endpoints is a length-prefixed, CRC-trailered
//! frame of **big-endian** `u32`s. Note big-endian — the *file* formats
//! ([`nord_format`]) are little-endian, and mixing them up is an easy afternoon lost.
//!
//! ```text
//! ┌────────┬─────────┬───────────┬─────────┬───────────────┬───────┐
//! │ length │ service │ subsystem │ command │ args…         │ crc16 │
//! │  u32   │   u32   │    u32    │   u32   │               │  u16  │
//! └────────┴─────────┴───────────┴─────────┴───────────────┴───────┘
//!   total inc. crc                          responses lead   over all
//!                                           with u32 status  preceding bytes
//! ```
//!
//! Verified against **every** message in the specimen corpus: 4,589 messages,
//! 100% CRC match and 100% length-field match.
//!
//! Requests carry an even `command`; the matching response is `command + 1` and
//! inserts a `u32` status (0 = success) ahead of the echoed arguments. That inserted
//! word is the reason responses run exactly 4 bytes longer than their requests.

use crate::error::{Error, Result};

/// Bytes ahead of the argument region: length, service, subsystem, command.
pub const HEADER_LEN: usize = 16;
/// Trailing CRC-16.
pub const CRC_LEN: usize = 2;

/// Functional area the message is addressed to.
///
/// Only two are observed so far. `Ui` carries the human-readable progress strings
/// NSM displays (`"Deleting..."`, `"Uploading..."`); `Program` is where the actual
/// work happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Service {
    /// Session control and UI progress strings. Pairs with subsystem `1`.
    Ui,
    /// Program/slot operations. Pairs with subsystem `10`.
    Program,
    Unknown(u32),
}

impl Service {
    pub fn from_raw(v: u32) -> Self {
        match v {
            6 => Service::Ui,
            12 => Service::Program,
            other => Service::Unknown(other),
        }
    }

    pub fn to_raw(self) -> u32 {
        match self {
            Service::Ui => 6,
            Service::Program => 12,
            Service::Unknown(v) => v,
        }
    }
}

/// Command codes observed on [`Service::Program`] (subsystem 10).
///
/// Requests are even and the response is always `+1`, so only the request code is
/// named. Codes are what the device actually sent — not guesses.
pub mod cmd {
    /// Open the transaction (the `O22 I26` that starts every operation).
    pub const SESSION_OPEN: u32 = 0x04;
    /// Close the transaction.
    pub const SESSION_CLOSE: u32 = 0x06;
    /// Device/memory status; the response carries several counters.
    pub const STATUS: u32 = 0x08;
    /// Delete a program.
    pub const DELETE: u32 = 0x14;
    /// Read a program's data. Response body is a reframed entity.
    pub const READ: u32 = 0x12;
    /// Move a program between slots.
    pub const MOVE: u32 = 0x18;
    /// Read a program's metadata (name, format tag).
    pub const INFO: u32 = 0x1e;
    /// Rename a program; args carry a length-prefixed string.
    pub const RENAME: u32 = 0x1c;

    /// Begin writing an entity. Args: bank, slot, body length, format tag,
    /// timestamp, `0xFFFFFFFF`, 1, and a trailing flag byte.
    pub const BEGIN_WRITE: u32 = 0x0a;
    /// Begin reading an entity. Args: bank, slot.
    pub const BEGIN_READ: u32 = 0x0c;
    /// Finish a transfer, either direction. Args: bank, slot.
    pub const END_TRANSFER: u32 = 0x0e;
    /// Push entity bytes. Args: bank, slot, offset, length, then the body.
    pub const WRITE_DATA: u32 = 0x10;
    /// List an entity's piano/sample dependencies.
    pub const DEPENDENCIES: u32 = 0x28;
}

/// What [`cmd::INFO`] reports about one slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramInfo {
    pub location: Location,
    /// Length of the entity body on the wire — 121 for an Electro 5 program.
    pub body_len: u32,
    /// Four-character CBIN format tag, e.g. `ne5p`.
    pub format: String,
    /// CRC-32 of the body, as the device reports it. Lets a read be verified
    /// against the device's own checksum rather than trusting the transfer.
    pub crc32: Option<u32>,
    /// Slot name as shown on the instrument.
    pub name: String,
}

impl ProgramInfo {
    pub fn decode(msg: &Message) -> Result<Self> {
        let p = msg.payload();
        if p.len() < 20 {
            return Err(Error::Truncated { got: p.len(), need: 20 });
        }
        let word = |i: usize| u32::from_be_bytes(p[i..i + 4].try_into().unwrap());
        let format = String::from_utf8_lossy(&p[12..16]).into_owned();

        // The name is a big-endian length-prefixed ASCII string. Its offset shifts
        // with the reply variant, so scan for a plausible length word rather than
        // hard-coding a position we have only seen one example of.
        let mut name = String::new();
        let mut crc32 = None;
        let mut i = 16;
        while i + 4 <= p.len() {
            let n = word(i) as usize;
            if n > 0 && n <= 32 && i + 4 + n <= p.len() && p[i + 4..i + 4 + n].is_ascii() {
                name = String::from_utf8_lossy(&p[i + 4..i + 4 + n]).trim_end().to_owned();
                break;
            }
            i += 4;
        }
        if p.len() >= 4 {
            crc32 = Some(u32::from_be_bytes(p[p.len() - 4..].try_into().unwrap()));
        }

        Ok(Self {
            location: Location { bank: word(0), slot: word(4) },
            body_len: word(8),
            format,
            crc32,
            name,
        })
    }
}

/// CRC-16/CCITT-FALSE — poly `0x1021`, init `0xFFFF`, no reflection, no xorout.
///
/// Identified by brute-forcing the standard CRC-16 parameter space against known
/// message/trailer pairs, then confirmed against all 4,589 corpus messages.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// One protocol message, decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub service: Service,
    pub subsystem: u32,
    pub command: u32,
    /// Everything between the command word and the CRC. For a response this still
    /// includes the leading status word — use [`Message::status`] to read it.
    pub args: Vec<u8>,
    /// Set by the decoder from the direction the bytes travelled. Not inferable from
    /// the command code — see [`Message::is_response`].
    is_response: bool,
}

impl Message {
    /// A request, to send to the device.
    pub fn new(service: Service, subsystem: u32, command: u32, args: Vec<u8>) -> Self {
        Self { service, subsystem, command, args, is_response: false }
    }

    /// Whether this message was decoded as a device response.
    ///
    /// **Direction, not parity.** An earlier version inferred this from the command
    /// being odd, which held for every op decoded at the time and is wrong in general:
    /// the "select in instrument" command is `0x2f` (odd) with response `0x30` (even),
    /// exactly inverting the guess. The `response == request + 1` rule does hold — only
    /// the parity of the request does not. Getting this backwards silently misaligns
    /// [`Self::payload`] by four bytes and hides device errors, so it is now recorded
    /// at decode time by the side that knows.
    pub fn is_response(&self) -> bool {
        self.is_response
    }

    /// The status word a response leads with. `Some(0)` is success.
    pub fn status(&self) -> Option<u32> {
        if !self.is_response || self.args.len() < 4 {
            return None;
        }
        Some(u32::from_be_bytes(self.args[..4].try_into().ok()?))
    }

    /// Arguments with the response status word stripped, so request and response
    /// argument lists line up.
    pub fn payload(&self) -> &[u8] {
        if self.is_response && self.args.len() >= 4 {
            &self.args[4..]
        } else {
            &self.args
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let len = (HEADER_LEN + self.args.len() + CRC_LEN) as u32;
        let mut out = Vec::with_capacity(len as usize);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&self.service.to_raw().to_be_bytes());
        out.extend_from_slice(&self.subsystem.to_be_bytes());
        out.extend_from_slice(&self.command.to_be_bytes());
        out.extend_from_slice(&self.args);
        out.extend_from_slice(&crc16(&out).to_be_bytes());
        out
    }

    /// Decode bytes received *from* the device.
    pub fn decode_response(buf: &[u8]) -> Result<Self> {
        let mut m = Self::decode(buf)?;
        m.is_response = true;
        Ok(m)
    }

    /// Decode bytes without asserting a direction; treated as a request.
    /// Prefer [`Self::decode_response`] for anything read off the wire.
    pub fn decode(buf: &[u8]) -> Result<Self> {
        if buf.len() < HEADER_LEN + CRC_LEN {
            return Err(Error::Truncated { got: buf.len(), need: HEADER_LEN + CRC_LEN });
        }
        let declared = u32::from_be_bytes(buf[0..4].try_into().unwrap()) as usize;
        if declared != buf.len() {
            return Err(Error::LengthMismatch { declared, actual: buf.len() });
        }

        let split = buf.len() - CRC_LEN;
        let expected = u16::from_be_bytes(buf[split..].try_into().unwrap());
        let actual = crc16(&buf[..split]);
        if expected != actual {
            return Err(Error::BadCrc { expected, actual });
        }

        Ok(Self {
            service: Service::from_raw(u32::from_be_bytes(buf[4..8].try_into().unwrap())),
            subsystem: u32::from_be_bytes(buf[8..12].try_into().unwrap()),
            command: u32::from_be_bytes(buf[12..16].try_into().unwrap()),
            args: buf[HEADER_LEN..split].to_vec(),
            is_response: false,
        })
    }
}

/// What kind of object a session is about.
///
/// `SESSION_OPEN` carries one of these, and [`cmd::STATUS`] then reports on that class
/// alone — which is why the same instrument reports different totals depending on what
/// was opened. Names are inferred from item counts cross-checked against a full backup
/// (29 pianos, 139 samples, ~375 programs); the numeric codes are what the device
/// actually sends, so an unrecognised one is preserved rather than rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectClass {
    Piano,
    Sample,
    Program,
    SetList,
    Unknown(u32),
}

impl ObjectClass {
    pub fn from_raw(v: u32) -> Self {
        match v {
            1 => ObjectClass::Piano,
            3 => ObjectClass::Sample,
            4 => ObjectClass::Program,
            5 => ObjectClass::SetList,
            other => ObjectClass::Unknown(other),
        }
    }

    pub fn to_raw(self) -> u32 {
        match self {
            ObjectClass::Piano => 1,
            ObjectClass::Sample => 3,
            ObjectClass::Program => 4,
            ObjectClass::SetList => 5,
            ObjectClass::Unknown(v) => v,
        }
    }

    /// The classes worth querying for an inventory. Codes 6 and 7 also answer, but
    /// reported zero items on every instrument seen so far.
    pub const INVENTORY: [ObjectClass; 4] =
        [ObjectClass::Piano, ObjectClass::Sample, ObjectClass::Program, ObjectClass::SetList];

    pub fn label(self) -> String {
        match self {
            ObjectClass::Piano => "pianos".into(),
            ObjectClass::Sample => "samples".into(),
            ObjectClass::Program => "programs".into(),
            ObjectClass::SetList => "set lists".into(),
            ObjectClass::Unknown(v) => format!("class {v}"),
        }
    }
}

/// What [`cmd::STATUS`] reports, for whichever [`ObjectClass`] the session opened.
///
/// `free + used` is constant per class (the class's total capacity). Deleting programs
/// was observed to raise `free` and lower `used` by the same amount, which is what
/// fixes the orientation — otherwise the two are indistinguishable.
///
/// The unit is not identified. It is *not* bytes: the program class totals 56,400,
/// which is far too small. Treat the numbers as opaque blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    pub class: ObjectClass,
    pub count: u32,
    pub free: u32,
    pub used: u32,
}

impl Status {
    pub fn total(&self) -> u32 {
        self.free + self.used
    }

    /// Blocks per item, when every item of this class costs the same.
    ///
    /// Fixed-size classes divide exactly: on a real Electro 5, programs cost **141**
    /// blocks each (`379 × 141 = 53439`, then `380 × 141 = 53580` after adding one)
    /// and set lists cost **38**. Variable-size classes — pianos and samples, where
    /// the content really does differ per item — do not divide evenly and yield
    /// `None`.
    pub fn blocks_per_item(&self) -> Option<u32> {
        if self.count == 0 || self.used == 0 || self.used % self.count != 0 {
            return None;
        }
        let per = self.used / self.count;
        // Only trust it if the class capacity is also a whole number of items;
        // otherwise the division is a coincidence.
        (per != 0 && self.total() % per == 0).then_some(per)
    }

    /// Total item slots, for classes where items are fixed-size.
    ///
    /// Far more meaningful than raw blocks: programs report 400, which is exactly the
    /// 8 banks × 50 slots of an Electro 5.
    pub fn slots(&self) -> Option<u32> {
        self.blocks_per_item().map(|per| self.total() / per)
    }

    pub fn used_percent(&self) -> f32 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            100.0 * self.used as f32 / total as f32
        }
    }

    /// Decode a [`cmd::STATUS`] response. Arguments after the status word are
    /// `count, free, used, …`.
    pub fn decode(class: ObjectClass, msg: &Message) -> Result<Self> {
        let p = msg.payload();
        if p.len() < 12 {
            return Err(Error::Truncated { got: p.len(), need: 12 });
        }
        let word = |i: usize| u32::from_be_bytes(p[i * 4..i * 4 + 4].try_into().unwrap());
        Ok(Self { class, count: word(0), free: word(1), used: word(2) })
    }
}

/// A bank/slot address. **Zero-indexed on the wire**, one-indexed in the UI and in
/// every capture directory name — `move_prog_8-13_to_7-16` puts `7, 12, 6, 15` on
/// the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub bank: u32,
    pub slot: u32,
}

impl Location {
    /// From the one-indexed numbering used by the UI and capture names.
    pub fn from_user(bank: u32, slot: u32) -> Self {
        Self { bank: bank - 1, slot: slot - 1 }
    }

    pub fn write_to(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.bank.to_be_bytes());
        out.extend_from_slice(&self.slot.to_be_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The middle exchange of `move_prog_8-13_to_7-16`, byte-for-byte off the wire.
    const MOVE: &str = "000000220000000c0000000a00000018000000070000000c000000060000000f4a55";
    /// Its response: command +1, status word inserted, arguments echoed.
    const MOVE_RESP: &str =
        "000000260000000c0000000a0000001900000000000000070000000c000000060000000f7197";

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn decodes_a_real_move() {
        let m = Message::decode(&hex(MOVE)).unwrap();
        assert_eq!(m.service, Service::Program);
        assert_eq!(m.subsystem, 10);
        assert_eq!(m.command, cmd::MOVE);
        assert!(!m.is_response());

        // 8-13 -> 7-16, zero-indexed on the wire.
        let mut want = Vec::new();
        Location::from_user(8, 13).write_to(&mut want);
        Location::from_user(7, 16).write_to(&mut want);
        assert_eq!(m.payload(), want.as_slice());
    }

    #[test]
    fn response_is_request_plus_one_plus_status() {
        let req = Message::decode(&hex(MOVE)).unwrap();
        let resp = Message::decode_response(&hex(MOVE_RESP)).unwrap();

        assert_eq!(resp.command, req.command + 1);
        assert!(resp.is_response());
        assert_eq!(resp.status(), Some(0));
        // Once the status word is stripped, the arguments are identical...
        assert_eq!(resp.payload(), req.payload());
        // ...which is exactly why responses run 4 bytes longer.
        assert_eq!(hex(MOVE_RESP).len() - hex(MOVE).len(), 4);
    }

    /// Direction cannot be inferred from the command code.
    ///
    /// "Select in instrument" is `0x2f` -> `0x30`: an **odd** request with an **even**
    /// response, inverting the parity guess that held for every other decoded op. Both
    /// messages are real, from `select_setlist_1-2` (set lists) and
    /// `open_on_device_2-12` (programs) -- the same command at two object classes.
    #[test]
    fn direction_is_not_inferable_from_command_parity() {
        // Request: cmd 0x2f, args (0, 1) -- displayed set list 1:2.
        let req = Message::decode(&hex("0000001a0000000c0000000a0000002f00000000000000017f71")).unwrap();
        assert_eq!(req.command, 0x2f);
        assert!(req.command & 1 == 1, "this request really is odd-numbered");
        assert!(!req.is_response(), "an odd command must still decode as a request");
        assert_eq!(req.status(), None);
        // A request's payload must not have four bytes eaten as a status word.
        assert_eq!(req.payload().len(), 8);

        // Response: cmd 0x30 (even), status 0, then the echoed args.
        let resp = Message::decode_response(
            &hex("0000001e0000000c0000000a0000003000000000000000000000000112c4"),
        )
        .unwrap();
        assert_eq!(resp.command, req.command + 1);
        assert!(resp.command & 1 == 0, "this response really is even-numbered");
        assert!(resp.is_response());
        assert_eq!(resp.status(), Some(0), "status must be readable despite even command");
        assert_eq!(resp.payload(), req.payload(), "args line up once status is stripped");
    }

    #[test]
    fn round_trips_byte_exact() {
        for raw in [MOVE, MOVE_RESP] {
            let bytes = hex(raw);
            assert_eq!(Message::decode(&bytes).unwrap().encode(), bytes);
        }
    }

    #[test]
    fn rejects_a_corrupted_crc() {
        let mut bytes = hex(MOVE);
        *bytes.last_mut().unwrap() ^= 0xFF;
        assert!(matches!(Message::decode(&bytes), Err(Error::BadCrc { .. })));
    }

    #[test]
    fn crc_matches_known_messages() {
        // Session open/close and the UI hello, straight from the corpus.
        for raw in [
            "0000001200000006000000010000000006a1",
            "000000160000000c0000000a0000000400000004a218",
            "000000120000000c0000000a000000066500",
        ] {
            assert!(Message::decode(&hex(raw)).is_ok(), "{raw}");
        }
    }
}
