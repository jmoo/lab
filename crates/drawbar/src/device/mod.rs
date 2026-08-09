//! The instrument: what the app knows about the attached Nord, and the channel that
//! talks to it.
//!
//! egui is immediate-mode and single-threaded; USB operations are slow and async. So
//! the UI never touches a transport — it sends a [`DeviceCmd`] to a worker that owns
//! one, and reads [`DeviceEvent`]s back. **One operation is in flight at a time**,
//! which is also all the protocol allows: a transaction is not re-entrant.

use std::collections::HashMap;
use std::sync::mpsc::Receiver;

use eframe::egui;
use nord_usb::wire::{Dependency, ProgramInfo, Status};
use nord_usb::{Location, ObjectClass};

use crate::log::Log;
use crate::workspace::{shown, Origin, Workspace};

mod panel;
mod worker;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
use native::Link;

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
use web::Link;

pub use worker::{Emit, Flow};

/// What the UI asks the instrument to do.
#[derive(Clone)]
pub enum DeviceCmd {
    Inventory,
    /// One `INFO` per slot of a bank, in a single session.
    ScanBank {
        class: ObjectClass,
        bank: u32,
        slots: u32,
    },
    SlotInfo {
        class: ObjectClass,
        at: Location,
    },
    Deps {
        class: ObjectClass,
        at: Location,
    },
    Get {
        class: ObjectClass,
        at: Location,
        /// The wire body verbatim, rather than a whole CBIN file.
        body: bool,
        /// A sweep capture's label, which names the workspace entity.
        label: Option<String>,
    },
    Put {
        class: ObjectClass,
        at: Location,
        name: String,
        bytes: Vec<u8>,
    },
    Move {
        class: ObjectClass,
        from: Location,
        to: Location,
    },
    Duplicate {
        class: ObjectClass,
        from: Location,
        to: Location,
    },
    Delete {
        class: ObjectClass,
        at: Location,
    },
    Rename {
        class: ObjectClass,
        at: Location,
        name: String,
    },
    Select {
        class: ObjectClass,
        at: Location,
    },
    Disconnect,
}

impl DeviceCmd {
    /// What the log and the in-flight spinner call this operation.
    pub fn label(&self) -> String {
        match self {
            DeviceCmd::Inventory => "inventory".into(),
            DeviceCmd::ScanBank { bank, .. } => format!("scan bank {bank}"),
            DeviceCmd::SlotInfo { at, .. } => format!("info {}", shown(*at)),
            DeviceCmd::Deps { at, .. } => format!("deps {}", shown(*at)),
            DeviceCmd::Get { at, body, .. } => match body {
                true => format!("get {} (raw body)", shown(*at)),
                false => format!("get {}", shown(*at)),
            },
            DeviceCmd::Put { at, name, .. } => format!("put {name} -> {}", shown(*at)),
            DeviceCmd::Move { from, to, .. } => {
                format!("move {} -> {}", shown(*from), shown(*to))
            }
            DeviceCmd::Duplicate { from, to, .. } => {
                format!("duplicate {} -> {}", shown(*from), shown(*to))
            }
            DeviceCmd::Delete { at, .. } => format!("delete {}", shown(*at)),
            DeviceCmd::Rename { at, name, .. } => format!("rename {} to {name:?}", shown(*at)),
            DeviceCmd::Select { at, .. } => format!("select {}", shown(*at)),
            DeviceCmd::Disconnect => "disconnect".into(),
        }
    }
}

/// What the worker reports back.
pub enum DeviceEvent {
    Connected(DeviceCard),
    ConnectFailed(String),
    Disconnected,
    Started(String),
    Finished,
    Inventory(Vec<Status>),
    BankScanned {
        class: ObjectClass,
        bank: u32,
        /// One entry per slot, `None` where the slot is vacant.
        slots: Vec<Option<ProgramInfo>>,
    },
    SlotInfo {
        class: ObjectClass,
        at: Location,
        info: Option<ProgramInfo>,
    },
    Deps {
        class: ObjectClass,
        at: Location,
        deps: Vec<Dependency>,
    },
    Got {
        name: String,
        origin: Origin,
        bytes: Vec<u8>,
    },
    /// A slot's former contents, which a failed write and a failed restore left with
    /// nowhere else to go.
    Rescued {
        at: Location,
        name: String,
        bytes: Vec<u8>,
    },
    Note(String),
    OpOk(String),
    OpFailed(String),
    InstrumentChanged,
}

/// The attached instrument, from its USB descriptors — answerable before any
/// transaction is opened.
#[derive(Clone)]
pub struct DeviceCard {
    pub product: String,
    pub manufacturer: Option<String>,
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial: Option<String>,
}

#[derive(Default)]
pub enum Connection {
    #[default]
    Disconnected,
    Connecting,
    Connected(DeviceCard),
}

/// One slot's detail, as the last `info`/`deps` reported it.
#[derive(Default)]
pub struct Detail {
    pub at: Option<Location>,
    pub info: Option<ProgramInfo>,
    /// Whether the last `info` said the slot was empty, as opposed to never asked.
    pub asked: bool,
    pub deps: Option<Vec<Dependency>>,
}

/// The UI's cache of the instrument. Nothing here is authoritative — it is what the
/// device last said, which [`DeviceState::stale`] flags as possibly out of date.
#[derive(Default)]
pub struct DeviceState {
    pub connection: Connection,
    /// The operation currently running, if any. One at a time.
    pub in_flight: Option<String>,
    pub inventory: Vec<Status>,
    /// The instrument reported a change made outside our session, so every cached
    /// slot name may now be wrong.
    pub stale: bool,
    banks: HashMap<(u32, u32), Vec<Option<ProgramInfo>>>,
    pub detail: Detail,
}

impl DeviceState {
    pub fn connected(&self) -> bool {
        matches!(self.connection, Connection::Connected(_))
    }

    /// A scanned bank's slots, or `None` if it has not been scanned.
    pub fn bank(&self, class: ObjectClass, bank: u32) -> Option<&[Option<ProgramInfo>]> {
        self.banks.get(&(class.to_raw(), bank)).map(Vec::as_slice)
    }

    /// What a slot holds, from the scan cache. `Some(None)` is a scanned empty slot.
    pub fn slot(&self, class: ObjectClass, at: Location) -> Option<Option<&ProgramInfo>> {
        let bank = self.bank(class, at.bank + 1)?;
        bank.get(at.slot as usize).map(Option::as_ref)
    }

    /// Drop one bank's cached names, because something just changed them.
    fn forget_bank(&mut self, class: ObjectClass, bank: u32) {
        self.banks.remove(&(class.to_raw(), bank));
    }

    fn forget_everything(&mut self) {
        self.banks.clear();
        self.inventory.clear();
        self.detail = Detail::default();
        self.stale = false;
    }
}

/// Slots per bank, per class.
///
/// ⚠️ The protocol reports a class's total slot count but never how the panel divides
/// it into banks. These are the Electro 5's divisions — 50 programs and 50 set lists to
/// a bank, three live slots, one settings singleton — inferred from the corpus and the
/// panel's own labels, not read off the device. A scan is not held to them: the device
/// answers status 3 past the end of its slot space, and that is where the walk stops.
pub fn slots_per_bank(class: ObjectClass) -> u32 {
    match class {
        ObjectClass::Live => 3,
        ObjectClass::Settings => 1,
        _ => 50,
    }
}

/// Whether a class accepts a write.
///
/// ⚠️ A write is a delete followed by a write, and whether the live buffer or the
/// settings singleton survives a delete of its own class is unconfirmed on hardware.
/// Until it is, an edit of either stops at a file.
pub fn put_refusal(class: ObjectClass) -> Option<String> {
    match class {
        ObjectClass::Live | ObjectClass::Settings => Some(format!(
            "writing {} back over USB is unproven on hardware; export the object as a \
             file instead",
            class.label(),
        )),
        _ => None,
    }
}

/// Turn a sweep label or a device-supplied slot name into something a file picker can
/// take later.
///
/// The label is prose — `split point C4`, `vol 5 -> 6` — and in the workspace it is the
/// only record of what the bytes mean, so it stays readable: whitespace runs become one
/// `-`, and only what a path cannot carry is dropped.
pub fn stem(label: &str) -> String {
    // A separator is owed rather than written, so a run of them collapses to one `-`
    // and nothing trailing survives.
    let mut owed = false;
    let mut out = String::with_capacity(label.len());
    for c in label.chars() {
        match c {
            '-' => owed = !out.is_empty(),
            _ if c.is_whitespace() => owed = !out.is_empty(),
            // Path separators, plus the characters a Windows filename cannot hold.
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => owed = !out.is_empty(),
            _ if c.is_control() => {}
            _ => {
                if std::mem::take(&mut owed) {
                    out.push('-');
                }
                out.push(c);
            }
        }
    }
    // A leading dot hides the file and dots alone spell `.` and `..`; a leading dash is
    // an option to every tool that later reads it.
    out.trim_matches(['.', '-']).to_string()
}

/// A pending mutation, waiting to be confirmed.
///
/// ⚠️ There is no armed mode. The only path to a destructive session is this struct:
/// the UI reads the victim, names it, and the worker escalates the session only for
/// the one command that comes out of here.
pub struct Confirm {
    pub title: String,
    pub lines: Vec<String>,
    cmd: DeviceCmd,
}

/// The instrument pane's own widget state — which class, which bank, what is selected.
struct Pane {
    tab: ObjectClass,
    other_class: u32,
    bank: u32,
    selected: Option<Location>,
    dest_bank: u32,
    dest_slot: u32,
    rename_to: String,
    sweep_label: String,
    confirm: Option<Confirm>,
}

impl Default for Pane {
    fn default() -> Pane {
        Pane {
            tab: ObjectClass::Program,
            // Pianos are class 1 — the class with no noun of its own.
            other_class: 1,
            bank: 1,
            selected: None,
            dest_bank: 1,
            dest_slot: 1,
            rename_to: String::new(),
            sweep_label: String::new(),
            confirm: None,
        }
    }
}

pub struct Device {
    pub state: DeviceState,
    events: Receiver<DeviceEvent>,
    link: Link,
    pane: Pane,
}

impl Device {
    pub fn new(ctx: egui::Context) -> Device {
        let (sender, events) = std::sync::mpsc::channel();
        Device {
            state: DeviceState::default(),
            events,
            link: Link::new(ctx, sender),
            pane: Pane::default(),
        }
    }

    /// ⚠️ Must be reached from the frame the button was clicked in. On the web the
    /// device chooser needs the click's transient user activation, and awaiting
    /// anything first spends it.
    pub fn connect(&mut self, log: &mut Log) {
        if !matches!(self.state.connection, Connection::Disconnected) {
            return;
        }
        self.state.connection = Connection::Connecting;
        log.info("connecting to the instrument");
        self.link.connect();
    }

    pub fn disconnect(&mut self, log: &mut Log) {
        if !self.state.connected() {
            return;
        }
        log.info("releasing the instrument");
        self.link.disconnect();
    }

    /// Queue one command. Refused while another is in flight — the protocol runs one
    /// transaction at a time, and the pane disables its buttons to match.
    pub fn send(&mut self, cmd: DeviceCmd, log: &mut Log) {
        if !self.state.connected() {
            log.error("not connected");
            return;
        }
        if let Some(running) = &self.state.in_flight {
            log.warn(format!(
                "{running} is still running; try again when it finishes"
            ));
            return;
        }
        // A mutation makes the names this pane is showing wrong, so they are dropped
        // rather than quietly kept — a stale name is what a confirmation would go on to
        // quote back at the operator.
        match &cmd {
            DeviceCmd::Delete { class, at }
            | DeviceCmd::Rename { class, at, .. }
            | DeviceCmd::Put { class, at, .. } => self.state.forget_bank(*class, at.bank + 1),
            DeviceCmd::Move { class, from, to } | DeviceCmd::Duplicate { class, from, to } => {
                self.state.forget_bank(*class, from.bank + 1);
                self.state.forget_bank(*class, to.bank + 1);
            }
            _ => {}
        }
        self.state.in_flight = Some(cmd.label());
        log.info(cmd.label());
        self.link.send(cmd);
    }

    /// Drain the worker's events into the cache and the workspace. Call once a frame.
    pub fn poll(&mut self, log: &mut Log, workspace: &mut Workspace) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                DeviceEvent::Connected(card) => {
                    log.info(format!(
                        "connected: {} ({:04x}:{:04x})",
                        card.product, card.vendor_id, card.product_id
                    ));
                    self.state.connection = Connection::Connected(card);
                    self.state.forget_everything();
                    // The first thing worth knowing, and read-only.
                    self.send(DeviceCmd::Inventory, log);
                }
                DeviceEvent::ConnectFailed(why) => {
                    log.error(why);
                    self.state.connection = Connection::Disconnected;
                }
                DeviceEvent::Disconnected => {
                    log.info("disconnected");
                    self.state.connection = Connection::Disconnected;
                    self.state.in_flight = None;
                    self.state.forget_everything();
                    self.pane.selected = None;
                }
                DeviceEvent::Started(what) => self.state.in_flight = Some(what),
                DeviceEvent::Finished => self.state.in_flight = None,
                DeviceEvent::Inventory(report) => self.state.inventory = report,
                DeviceEvent::BankScanned { class, bank, slots } => {
                    self.state.banks.insert((class.to_raw(), bank), slots);
                }
                DeviceEvent::SlotInfo { at, info, .. } => {
                    self.state.detail = Detail {
                        at: Some(at),
                        info,
                        asked: true,
                        deps: None,
                    };
                }
                DeviceEvent::Deps { at, deps, .. } => {
                    if self.state.detail.at != Some(at) {
                        self.state.detail = Detail {
                            at: Some(at),
                            ..Detail::default()
                        };
                    }
                    self.state.detail.deps = Some(deps);
                }
                DeviceEvent::Got {
                    name,
                    origin,
                    bytes,
                } => {
                    workspace.ingest(name, origin, bytes, log);
                }
                DeviceEvent::Rescued { at, name, bytes } => {
                    log.error(format!(
                        "{} could not be restored; its bytes are in the workspace as {name}",
                        shown(at)
                    ));
                    workspace.ingest(name, Origin::Rescued { at }, bytes, log);
                }
                DeviceEvent::Note(text) => log.info(text),
                DeviceEvent::OpOk(text) => log.info(text),
                DeviceEvent::OpFailed(text) => log.error(text),
                DeviceEvent::InstrumentChanged => {
                    self.state.stale = true;
                    log.warn("the instrument changed under us — cached slot names may be stale");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The label is the only description the workspace will ever have of a swept
    /// capture, so it survives into the name rather than being reduced to something
    /// opaque.
    #[test]
    fn a_swept_capture_keeps_the_words_it_was_described_with() {
        assert_eq!(stem("split point C4"), "split-point-C4");
        assert_eq!(stem("  transpose +1  "), "transpose-+1");
        assert_eq!(stem("organ vol 5 -> 6"), "organ-vol-5-6");
    }

    /// The name reaches a file picker later, so nothing in it may be a path.
    #[test]
    fn a_label_cannot_become_a_path() {
        assert_eq!(stem("../../etc/passwd"), "etc-passwd");
        assert_eq!(stem("rotary:fast"), "rotary-fast");
        assert_eq!(stem(".hidden"), "hidden");
    }

    /// Nothing usable left is reported as nothing, for the caller to refuse or
    /// substitute — never silently turned into a default.
    #[test]
    fn a_label_with_no_name_in_it_comes_back_empty() {
        for bad in ["...", "/", "  ", "?*", "-"] {
            assert!(stem(bad).is_empty(), "{bad:?}");
        }
    }

    /// The two classes whose write path is unproven must refuse by name, and the rest
    /// must not.
    #[test]
    fn only_live_and_settings_refuse_a_put() {
        for class in [ObjectClass::Live, ObjectClass::Settings] {
            let why = put_refusal(class).expect("must refuse");
            assert!(why.contains("unproven on hardware"), "{why}");
        }
        for class in ObjectClass::INVENTORY {
            assert!(put_refusal(class).is_none(), "{}", class.label());
        }
        assert!(put_refusal(ObjectClass::Unknown(9)).is_none());
    }
}
