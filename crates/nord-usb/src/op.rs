//! Typed operations.
//!
//! Batching is a property of the operation, not of a generic helper: most ops repeat
//! their per-item unit N times, but `duplicate` is **phase-separated** — all N
//! addressing exchanges first, then all N data-copy pairs. Encoding that per-op beats
//! a generic loop that then needs an escape hatch.

use crate::error::{Error, Result};
use crate::session::Session;
use crate::transport::Transport;
use crate::envelope;
use crate::session::ReadWrite;
use crate::wire::{cmd, Location, ObjectClass, ProgramInfo, Service, Status};

/// Query the inventory for the class the session was opened with.
///
/// **Read-only.** It sends one request and reads counters back; nothing on the
/// instrument changes. That makes it the safe way to prove the whole stack works
/// against real hardware.
pub async fn status<T: Transport, C>(session: &mut Session<'_, T, C>) -> Result<Status> {
    let class = session.class();
    let resp =
        session.request(Service::Program, 10, cmd::STATUS, &class.to_raw().to_be_bytes()).await?;
    Status::decode(class, &resp)
}

/// Query every class worth reporting, one transaction each.
///
/// Each class needs its own session because the class is fixed at `SESSION_OPEN`.
/// A class that errors is skipped rather than failing the sweep — instruments differ
/// in which classes they answer for.
pub async fn inventory<T: Transport>(transport: &mut T) -> Result<Vec<Status>> {
    let mut out = Vec::new();
    for class in ObjectClass::INVENTORY {
        let mut session = match Session::open(transport, class).await {
            Ok(s) => s,
            Err(_) => continue,
        };
        match status(&mut session).await {
            Ok(s) => {
                session.commit().await?;
                out.push(s);
            }
            Err(_) => session.abort(),
        }
    }
    Ok(out)
}

/// Ask the device about one slot: format tag, body length, name, body checksum.
///
/// **Read-only.**
pub async fn info<T: Transport, C>(
    session: &mut Session<'_, T, C>,
    at: Location,
) -> Result<ProgramInfo> {
    let mut args = Vec::new();
    at.write_to(&mut args);
    let resp = session.request(Service::Program, 10, cmd::INFO, &args).await?;
    ProgramInfo::decode(&resp)
}

/// Read one program off the instrument, returning the bytes of a `.ne5p` file.
///
/// **Read-only.** Follows the sequence NSM uses: `INFO` to learn the body length,
/// a progress string, `BEGIN_READ`, `READ`, then `END_TRANSFER`. The body is wrapped
/// in a `CBIN` header ([`envelope`]) so the result is a real file, and the device's
/// own CRC-32 is checked against it when the device supplies one.
pub async fn read_program<T: Transport, C>(
    session: &mut Session<'_, T, C>,
    at: Location,
) -> Result<Vec<u8>> {
    let meta = info(session, at).await?;

    let mut args = Vec::new();
    at.write_to(&mut args);
    session.request(Service::Program, 10, cmd::BEGIN_READ, &args).await?;

    let mut req = args.clone();
    req.extend_from_slice(&0u32.to_be_bytes()); // offset
    req.extend_from_slice(&meta.body_len.to_be_bytes());
    let resp = session.request(Service::Program, 10, cmd::READ, &req).await?;

    // Payload is bank, slot, offset, length, then the body.
    let p = resp.payload();
    let body = p.get(16..).ok_or(Error::Truncated { got: p.len(), need: 17 })?;
    if body.len() != meta.body_len as usize {
        return Err(Error::Transport(format!(
            "device announced a {}-byte body but sent {}",
            meta.body_len,
            body.len()
        )));
    }

    session.request(Service::Program, 10, cmd::END_TRANSFER, &args).await?;

    let file = envelope::wrap(&meta.format, at, body)?;
    if let Some(expected) = meta.crc32 {
        let (_, _, wrapped) = envelope::unwrap(&file)?;
        let actual = crc32_of(wrapped);
        if expected != actual {
            return Err(Error::Transport(format!(
                "body checksum mismatch: device reported {expected:08x}, received {actual:08x}"
            )));
        }
    }
    Ok(file)
}

/// Write a `.ne5p` file into a slot, **overwriting whatever is there**.
///
/// Requires a [`ReadWrite`] session, which callers must obtain deliberately.
///
/// ⚠️ The `BEGIN_WRITE` argument layout is only partly understood: the fourth word is
/// a Unix timestamp (NSM sends the file's mtime) and the trailing bytes are copied
/// from an observed capture. This has been validated against a recorded exchange but
/// **not yet against real hardware**. Back up before using it.
pub async fn write_program<T: Transport>(
    session: &mut Session<'_, T, ReadWrite>,
    at: Location,
    file: &[u8],
    timestamp: u32,
) -> Result<()> {
    let (format, _, body) = envelope::unwrap(file)?;

    let mut begin = Vec::new();
    at.write_to(&mut begin);
    begin.extend_from_slice(&(body.len() as u32).to_be_bytes());
    begin.extend_from_slice(format.as_bytes());
    begin.extend_from_slice(&timestamp.to_be_bytes());
    begin.extend_from_slice(&u32::MAX.to_be_bytes());
    begin.extend_from_slice(&1u32.to_be_bytes());
    begin.push(b'0'); // trailing flag byte, copied from the observed capture
    session.request(Service::Program, 10, cmd::BEGIN_WRITE, &begin).await?;

    let mut data = Vec::new();
    at.write_to(&mut data);
    data.extend_from_slice(&0u32.to_be_bytes()); // offset
    data.extend_from_slice(&(body.len() as u32).to_be_bytes());
    data.extend_from_slice(body);
    session.request(Service::Program, 10, cmd::WRITE_DATA, &data).await?;

    let mut args = Vec::new();
    at.write_to(&mut args);
    session.request(Service::Program, 10, cmd::END_TRANSFER, &args).await?;
    Ok(())
}

// NSM also sends a UI progress string ("Uploading..." / "Downloading...") at the
// start of each transfer. We deliberately do NOT.
//
// They are cosmetic — they drive the instrument's display — and they are
// fire-and-forget, so the device never acknowledges them and cannot be waiting on
// one. More to the point, the two captured examples do not agree on a single frame
// layout (one is 38 bytes declaring 37, the other 39 declaring 39), so any encoder
// written from them would be guesswork. Sending nothing is well-defined; sending a
// malformed frame is not.
//
// If hardware turns out to want them, re-capture a clean example first.

fn crc32_of(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

// ---------------------------------------------------------------------------
// Declared but not yet implemented. The exchange shapes are verified against the
// corpus and the wire encoding is known (see `wire::cmd`), but these mutate the
// device, so they land behind the ReadWrite capability once there is a backup story.
// ---------------------------------------------------------------------------

/// Move a program. Verified shape: `O34 I38`.
#[derive(Debug)]
pub struct Move {
    pub from: Location,
    pub to: Location,
}

/// Delete programs. Verified shape: `O36 O26 I30`, repeated per item.
#[derive(Debug)]
pub struct Delete {
    pub items: Vec<Location>,
}

/// Rename a program. Verified shape: `O33 I30`; the request carries a length-prefixed
/// name, so its size tracks the string.
#[derive(Debug)]
pub struct Rename {
    pub at: Location,
    pub name: String,
}

/// Duplicate programs. The one confirmed **phase-separated** operation.
#[derive(Debug)]
pub struct Duplicate {
    pub pairs: Vec<(Location, Location)>,
}
