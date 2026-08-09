//! The operations, run against whichever transport the target supplies.
//!
//! Everything here is generic over [`Transport`], so the browser and the desktop run
//! the same code and only the spawn glue is cfg'd.
//!
//! ⚠️ **Every session commits, including on the error path.** An abandoned transaction
//! leaves the instrument mid-operation with its progress label still painted, and the
//! only way out is a power cycle. A `?` between opening a session and committing it is
//! how that happens, so each operation below holds the result, commits, and only then
//! reports.

use std::sync::mpsc::Sender;

use eframe::egui;
use nord_usb::transport::Transport;
use nord_usb::wire::{Dependency, ProgramInfo, Status};
use nord_usb::{op, Error, Location, ObjectClass, Session};

use super::{DeviceCmd, DeviceEvent};
use crate::workspace::{shown, Origin};

/// The event channel back to the UI thread, with the repaint that makes an event
/// visible before the next input arrives.
#[derive(Clone)]
pub struct Emit {
    tx: Sender<DeviceEvent>,
    ctx: egui::Context,
}

impl Emit {
    pub fn new(tx: Sender<DeviceEvent>, ctx: egui::Context) -> Emit {
        Emit { tx, ctx }
    }

    pub fn send(&self, event: DeviceEvent) {
        let _ = self.tx.send(event);
        self.ctx.request_repaint();
    }
}

/// Whether the worker keeps its transport after this command.
#[derive(PartialEq, Eq)]
pub enum Flow {
    Continue,
    Stop,
}

/// Run one command to completion.
///
/// Emits exactly one [`DeviceEvent::Started`] and one [`DeviceEvent::Finished`], so the
/// UI's in-flight marker cannot be left set by an operation that failed halfway.
pub async fn run<T: Transport>(transport: &mut T, cmd: DeviceCmd, emit: &Emit) -> Flow {
    if matches!(cmd, DeviceCmd::Disconnect) {
        return Flow::Stop;
    }
    let what = cmd.label();
    emit.send(DeviceEvent::Started(what.clone()));

    let mut changed = false;
    let result = execute(transport, cmd, emit, &mut changed).await;

    // Reported before the outcome: state read during this command may already be stale,
    // and that is true whether it succeeded or not.
    if changed {
        emit.send(DeviceEvent::InstrumentChanged);
    }
    match result {
        Ok(Some(note)) => emit.send(DeviceEvent::OpOk(note)),
        Ok(None) => {}
        Err(e) => emit.send(DeviceEvent::OpFailed(format!("{what}: {e}"))),
    }
    emit.send(DeviceEvent::Finished);
    Flow::Continue
}

/// The command bodies. `Ok(Some(note))` is a line for the log; `Ok(None)` means the
/// command's own event already said everything.
async fn execute<T: Transport>(
    t: &mut T,
    cmd: DeviceCmd,
    emit: &Emit,
    changed: &mut bool,
) -> Result<Option<String>, String> {
    match cmd {
        // Handled by `run`; the transport is closed by the caller, which owns it.
        DeviceCmd::Disconnect => Ok(None),

        DeviceCmd::Inventory => {
            let report = op::inventory(t).await.map_err(|e| e.to_string())?;
            // Not merely an empty inventory: every class is queried, so nothing coming
            // back means no class answered at all.
            if report.is_empty() {
                return Err(
                    "no object class answered — either the instrument is not in a \
                            usable session state (a power cycle clears it), or the \
                            connection failed"
                        .into(),
                );
            }
            let note = format!("inventory: {} classes answered", report.len());
            emit.send(DeviceEvent::Inventory(report));
            Ok(Some(note))
        }

        DeviceCmd::ScanBank {
            class,
            bank,
            slots: count,
        } => {
            let slots = scan_bank(t, class, bank, count, changed)
                .await
                .map_err(|e| e.to_string())?;
            let filled = slots.iter().filter(|s| s.is_some()).count();
            let note = format!(
                "bank {bank}: {filled} of {} slots hold something",
                slots.len()
            );
            emit.send(DeviceEvent::BankScanned { class, bank, slots });
            Ok(Some(note))
        }

        DeviceCmd::SlotInfo { class, at } => {
            let info = match slot_info(t, class, at, changed).await {
                Ok(info) => Some(info),
                // Status 1 is a vacant slot, not a failure.
                Err(Error::DeviceStatus(1)) => None,
                Err(e) => return Err(explain(e, at)),
            };
            emit.send(DeviceEvent::SlotInfo { class, at, info });
            Ok(None)
        }

        DeviceCmd::Deps { class, at } => {
            let deps = dependencies(t, class, at, changed)
                .await
                .map_err(|e| explain(e, at))?;
            let note = format!("{}: {} dependencies", shown(at), deps.len());
            emit.send(DeviceEvent::Deps { class, at, deps });
            Ok(Some(note))
        }

        DeviceCmd::Get {
            class,
            at,
            body,
            label,
        } => {
            let (info, bytes) = read_object(t, class, at, body, changed)
                .await
                .map_err(|e| explain(e, at))?;
            let note = format!(
                "read {:?} from {} ({} bytes)",
                info.name,
                shown(at),
                bytes.len()
            );
            let origin = match &label {
                Some(label) => Origin::Sweep {
                    label: label.clone(),
                },
                None => Origin::Device { class, at },
            };
            emit.send(DeviceEvent::Got {
                name: entity_name(&info, body, label.as_deref()),
                origin,
                bytes,
            });
            Ok(Some(note))
        }

        DeviceCmd::Put {
            class,
            at,
            name,
            bytes,
        } => put(t, class, at, &name, bytes, emit, changed).await,

        DeviceCmd::Select { class, at } => {
            select(t, class, at, changed)
                .await
                .map_err(|e| explain(e, at))?;
            Ok(Some(format!("selected {} on the instrument", shown(at))))
        }

        DeviceCmd::Rename { class, at, name } => {
            rename(t, class, at, &name, changed)
                .await
                .map_err(|e| explain(e, at))?;
            Ok(Some(format!("renamed {} to {name:?}", shown(at))))
        }

        DeviceCmd::Move { class, from, to } => {
            move_object(t, class, from, to, changed)
                .await
                .map_err(|e| explain(e, from))?;
            Ok(Some(format!("moved {} -> {}", shown(from), shown(to))))
        }

        DeviceCmd::Duplicate { class, from, to } => {
            duplicate(t, class, from, to, changed)
                .await
                .map_err(|e| explain(e, from))?;
            Ok(Some(format!("duplicated {} -> {}", shown(from), shown(to))))
        }

        DeviceCmd::Delete { class, at } => {
            delete(t, class, at, changed)
                .await
                .map_err(|e| explain(e, at))?;
            Ok(Some(format!("deleted {}", shown(at))))
        }
    }
}

/// Write bytes into a slot, replacing whatever is there.
///
/// ⚠️ **An occupied destination is replaced, not overwritten.** The instrument answers
/// status 4 to a write aimed at a slot that already holds something, so the occupant is
/// read, deleted, and put back if the write fails — the slot is genuinely empty in
/// between, and the only copy of its contents is in this process's memory. If the
/// restore fails too, those bytes leave here as [`DeviceEvent::Rescued`] rather than
/// being dropped on the floor.
async fn put<T: Transport>(
    t: &mut T,
    class: ObjectClass,
    at: Location,
    what: &str,
    bytes: Vec<u8>,
    emit: &Emit,
    changed: &mut bool,
) -> Result<Option<String>, String> {
    let existing = match slot_info(t, class, at, changed).await {
        Ok(info) => Some(info),
        Err(Error::DeviceStatus(1)) => None,
        Err(e) => return Err(explain(e, at)),
    };

    // Nothing is deleted until the backup is in hand.
    let backup = match &existing {
        Some(_) => Some(
            read_object(t, class, at, false, changed)
                .await
                .map(|(_, file)| file)
                .map_err(|e| {
                    format!(
                        "could not read {} back before replacing it, so it was left alone: {}",
                        shown(at),
                        explain(e, at)
                    )
                })?,
        ),
        None => None,
    };

    if existing.is_some() {
        emit.send(DeviceEvent::Note(format!(
            "deleting {} to make room",
            shown(at)
        )));
        delete(t, class, at, changed)
            .await
            .map_err(|e| format!("deleting {}: {}", shown(at), explain(e, at)))?;
    }

    let timestamp = unix_now();
    let written = write(t, class, at, &bytes, timestamp, changed).await;

    match (written, backup) {
        (Ok(()), _) => Ok(Some(format!("wrote {what} -> {}", shown(at)))),
        (Err(e), None) => Err(explain(e, at)),
        // Getting the occupant back matters more than reporting the original error,
        // which is carried along and reported once the slot is whole again.
        (Err(e), Some(backup)) => {
            emit.send(DeviceEvent::OpFailed(format!(
                "the write failed and {} is now empty; putting the original back",
                shown(at)
            )));
            match write(t, class, at, &backup, timestamp, changed).await {
                Ok(()) => Err(format!(
                    "{e} ({} was restored, and is unchanged)",
                    shown(at)
                )),
                Err(restore) => {
                    let name = rescue_name(at, &backup);
                    emit.send(DeviceEvent::Rescued {
                        at,
                        name,
                        bytes: backup,
                    });
                    Err(format!(
                        "{e} (restoring failed as well: {restore}); {} is EMPTY, and its \
                         former contents are now in the workspace as a rescued entity — \
                         put it back",
                        shown(at)
                    ))
                }
            }
        }
    }
}

/// Combine an operation's result with its session close, keeping the operation's error
/// when both fail — a close failing is usually a *consequence* of the op failing, and
/// the original error is the informative one.
fn finish<T>(result: Result<T, Error>, closed: Result<(), Error>) -> Result<T, Error> {
    match result {
        Ok(v) => closed.map(|()| v),
        Err(e) => Err(e),
    }
}

/// Turn the device's bare status code into something actionable.
///
/// All three confirmed on hardware: `0x1` from a vacant slot, `0x3` from a slot outside
/// the instrument's range, `0x4` from a write aimed at an occupied slot.
fn explain(e: Error, at: Location) -> String {
    match e {
        Error::DeviceStatus(1) => format!("{} is empty", shown(at)),
        Error::DeviceStatus(3) => format!("{} is out of range for this instrument", shown(at)),
        Error::DeviceStatus(4) => format!(
            "{} is occupied, and the instrument does not overwrite in place",
            shown(at)
        ),
        other => other.to_string(),
    }
}

async fn slot_info<T: Transport>(
    t: &mut T,
    class: ObjectClass,
    at: Location,
    changed: &mut bool,
) -> Result<ProgramInfo, Error> {
    let mut s = Session::open(t, class).await?;
    let r = op::info(&mut s, at).await;
    *changed |= s.instrument_changed();
    let closed = s.commit().await;
    finish(r, closed)
}

/// Every slot of one bank, in one session — the way the protocol expects a caller to
/// batch: open once, loop the per-item primitive.
///
/// A vacant slot is a `None` row rather than an error, and the walk stops where the
/// device says the class's slot space ends. That matters because nothing on the wire
/// reports how the panel divides a class into banks.
async fn scan_bank<T: Transport>(
    t: &mut T,
    class: ObjectClass,
    bank: u32,
    slots: u32,
    changed: &mut bool,
) -> Result<Vec<Option<ProgramInfo>>, Error> {
    let mut s = Session::open(t, class).await?;
    let mut out = Vec::new();
    let mut fatal = None;
    for slot in 1..=slots {
        // A refusal keeps the session in step — request and reply still pair — so the
        // walk continues inside the same transaction.
        match op::info(&mut s, Location::from_user(bank, slot)).await {
            Ok(info) => out.push(Some(info)),
            Err(Error::DeviceStatus(1)) => out.push(None),
            Err(Error::DeviceStatus(3)) => break,
            Err(e) => {
                fatal = Some(e);
                break;
            }
        }
    }
    *changed |= s.instrument_changed();
    let closed = s.commit().await;
    match fatal {
        Some(e) => Err(e),
        None => closed.map(|()| out),
    }
}

/// One read in its own session: the slot's metadata, then its bytes.
///
/// `body` returns the wire body verbatim; otherwise the bytes are a whole CBIN file.
async fn read_object<T: Transport>(
    t: &mut T,
    class: ObjectClass,
    at: Location,
    body: bool,
    changed: &mut bool,
) -> Result<(ProgramInfo, Vec<u8>), Error> {
    let mut s = Session::open(t, class).await?;
    let r = async {
        let info = op::info(&mut s, at).await?;
        let file = if body {
            op::read_body(&mut s, at).await?
        } else {
            op::read_program(&mut s, at).await?
        };
        Ok::<_, Error>((info, file))
    }
    .await;
    *changed |= s.instrument_changed();
    let closed = s.commit().await;
    finish(r, closed)
}

async fn dependencies<T: Transport>(
    t: &mut T,
    class: ObjectClass,
    at: Location,
    changed: &mut bool,
) -> Result<Vec<Dependency>, Error> {
    let mut s = Session::open(t, class).await?;
    let r = op::dependencies(&mut s, at).await;
    *changed |= s.instrument_changed();
    let closed = s.commit().await;
    finish(r, closed)
}

async fn select<T: Transport>(
    t: &mut T,
    class: ObjectClass,
    at: Location,
    changed: &mut bool,
) -> Result<(), Error> {
    let mut s = Session::open(t, class).await?;
    let r = op::select(&mut s, at).await;
    *changed |= s.instrument_changed();
    r.and(s.commit().await)
}

async fn rename<T: Transport>(
    t: &mut T,
    class: ObjectClass,
    at: Location,
    name: &str,
    changed: &mut bool,
) -> Result<(), Error> {
    let mut s = Session::open(t, class).await?.allow_destructive_writes();
    let r = op::rename(&mut s, at, name).await;
    *changed |= s.instrument_changed();
    r.and(s.commit().await)
}

async fn move_object<T: Transport>(
    t: &mut T,
    class: ObjectClass,
    from: Location,
    to: Location,
    changed: &mut bool,
) -> Result<(), Error> {
    let mut s = Session::open(t, class).await?.allow_destructive_writes();
    let r = op::move_object(&mut s, from, to).await;
    *changed |= s.instrument_changed();
    r.and(s.commit().await)
}

async fn duplicate<T: Transport>(
    t: &mut T,
    class: ObjectClass,
    from: Location,
    to: Location,
    changed: &mut bool,
) -> Result<(), Error> {
    let mut s = Session::open(t, class).await?.allow_destructive_writes();
    let r = op::duplicate(&mut s, from, to).await;
    *changed |= s.instrument_changed();
    r.and(s.commit().await)
}

async fn delete<T: Transport>(
    t: &mut T,
    class: ObjectClass,
    at: Location,
    changed: &mut bool,
) -> Result<(), Error> {
    let mut s = Session::open(t, class).await?.allow_destructive_writes();
    let r = op::delete(&mut s, at).await;
    *changed |= s.instrument_changed();
    r.and(s.commit().await)
}

async fn write<T: Transport>(
    t: &mut T,
    class: ObjectClass,
    at: Location,
    file: &[u8],
    timestamp: u32,
    changed: &mut bool,
) -> Result<(), Error> {
    let mut s = Session::open(t, class).await?.allow_destructive_writes();
    let r = op::write_program(&mut s, at, file, timestamp).await;
    *changed |= s.instrument_changed();
    r.and(s.commit().await)
}

/// Unix seconds, for the timestamp word `BEGIN_WRITE` carries.
///
/// ⚠️ `SystemTime::now()` traps on `wasm32-unknown-unknown`, so the browser's own clock
/// is what the web build reads.
#[cfg(not(target_arch = "wasm32"))]
fn unix_now() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

#[cfg(target_arch = "wasm32")]
fn unix_now() -> u32 {
    (js_sys::Date::now() / 1000.0) as u32
}

/// What the workspace calls an object read off the instrument.
///
/// Files store no name — it lives on the instrument — so a read is the one moment the
/// name and the bytes are together, and it goes into the entity's label here.
fn entity_name(info: &ProgramInfo, body: bool, label: Option<&str>) -> String {
    let stem = match label {
        Some(label) => super::stem(label),
        None => sanitise(info.name.trim()),
    };
    // A `--body` dump is a fragment of a file, not one; giving it the format's own
    // extension would invite it back in as a whole object.
    match body {
        true => format!("{stem}.body"),
        false => format!("{stem}.{}", info.format),
    }
}

/// Filename for a rescued slot: the location as the instrument labels it, and the
/// object's own format tag so it can be handed straight back to a put.
///
/// ⚠️ The tag is read out of the header rather than through `envelope::unwrap`, which
/// also verifies the checksum. These bytes are the last copy of the slot even if they
/// fail that check, so naming them must not depend on it.
fn rescue_name(at: Location, backup: &[u8]) -> String {
    let format = backup
        .get(8..12)
        .filter(|tag| tag.iter().all(|b| b.is_ascii_alphanumeric()))
        .map(|tag| String::from_utf8_lossy(tag).into_owned())
        .unwrap_or_else(|| "bin".to_string());
    format!("nord-rescued-{}-{}.{format}", at.bank + 1, at.slot + 1)
}

/// A device-supplied name, made safe to hand to a file picker later.
fn sanitise(name: &str) -> String {
    let cleaned = super::stem(name);
    match cleaned.is_empty() {
        true => "unnamed".into(),
        false => cleaned,
    }
}

/// A one-line description of what a class's inventory row means, for the pane header.
pub fn describe(status: &Status) -> String {
    match status.slots() {
        // Fixed-size classes are far clearer as slots than as raw blocks: programs
        // report 400, which is exactly an Electro 5's 8 banks x 50.
        Some(slots) => format!(
            "{} of {slots} slots, {:.1}% used ({}/{} blocks)",
            status.count,
            status.used_percent(),
            status.used,
            status.total(),
        ),
        None => format!(
            "{} items, {:.1}% used ({}/{} blocks)",
            status.count,
            status.used_percent(),
            status.used,
            status.total(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rescue entity is the last copy of a program that no longer exists on the
    /// instrument, so it has to be named something a person can act on.
    #[test]
    fn a_rescued_slot_is_named_for_its_location_and_format() {
        let mut file = vec![0u8; 45];
        file[0..4].copy_from_slice(b"CBIN");
        file[4..8].copy_from_slice(&1u32.to_le_bytes());
        file[8..12].copy_from_slice(b"ne5p");
        // Wire is zero-indexed, the instrument's labels are not.
        let at = Location { bank: 6, slot: 49 };
        assert_eq!(rescue_name(at, &file), "nord-rescued-7-50.ne5p");
    }

    /// Bytes that do not parse are still the only copy, so they still get a name.
    #[test]
    fn unparseable_bytes_still_get_rescued() {
        let at = Location { bank: 0, slot: 0 };
        assert_eq!(rescue_name(at, b"nonsense"), "nord-rescued-1-1.bin");
    }

    #[test]
    fn a_read_is_named_for_the_slot_name_and_the_devices_format_tag() {
        let info = ProgramInfo {
            location: Location { bank: 6, slot: 3 },
            body_len: 121,
            format: "ne5p".into(),
            version: 4,
            crc32: Some(0),
            name: "Africa Split".into(),
        };
        assert_eq!(entity_name(&info, false, None), "Africa-Split.ne5p");
        assert_eq!(entity_name(&info, true, None), "Africa-Split.body");
        assert_eq!(
            entity_name(&info, false, Some("split point C4")),
            "split-point-C4.ne5p"
        );
    }

    /// A slot whose name is only punctuation still has to produce a usable label.
    #[test]
    fn a_nameless_slot_still_gets_a_label() {
        let info = ProgramInfo {
            location: Location { bank: 0, slot: 0 },
            body_len: 121,
            format: "ne5p".into(),
            version: 4,
            crc32: None,
            name: "  ".into(),
        };
        assert_eq!(entity_name(&info, false, None), "unnamed.ne5p");
    }
}
