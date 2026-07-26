//! Typed operations.
//!
//! Batching is a property of the operation, not of a generic helper: most ops repeat
//! their per-item unit N times, but `duplicate` is **phase-separated** — all N
//! addressing exchanges first, then all N data-copy pairs. Encoding that per-op beats
//! a generic loop that then needs an escape hatch.

use crate::error::Result;
use crate::session::Session;
use crate::transport::Transport;
use crate::wire::{cmd, Location, ObjectClass, Service, Status};

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

// ---------------------------------------------------------------------------
// Declared but not yet implemented. The exchange shapes are verified against the
// corpus and the wire encoding is known (see `wire::cmd`), but these mutate the
// device, so they land behind the ReadWrite capability once there is a backup story.
// ---------------------------------------------------------------------------

/// Read a program. Verified shape: `O26 I30, O34 I159`, where the 159-byte response is
/// a reframed entity — 36-byte header, the same 121-byte body the `.ne5p` carries,
/// 2-byte CRC.
#[derive(Debug)]
pub struct Read {
    pub at: Location,
}

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
