//! The instrument: what the app knows about the attached Nord, and the channel that
//! talks to it.
//!
//! egui is immediate-mode and single-threaded; USB operations are slow and async. So
//! the UI never touches a transport — it sends a [`DeviceCmd`] to a worker that owns
//! one, and reads [`DeviceEvent`]s back. **One operation is in flight at a time**,
//! which is also all the protocol allows: a transaction is not re-entrant.
//!
//! Two queues feed that one slot. What the user asked for goes in `pending` and is
//! always dispatched first; the background read of every bank of every class waits in
//! [`Scan`] behind it, so browsing the tree never makes a click wait on it.

use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::Receiver;

use eframe::egui;
use nord_usb::wire::{Dependency, ProgramInfo, Status};
use nord_usb::{Location, ObjectClass};

use crate::log::Log;
use crate::strings::{folder, place, shown};
use crate::tabs::Tabs;
use crate::workspace::{Origin, Workspace};

mod scan;
mod worker;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
use native::Link;

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
use web::Link;

pub use scan::{Progress, Scan};
pub use worker::{Emit, Flow};

/// The folders the browser shows, in the order it shows them.
pub const BROWSED: [ObjectClass; 6] = [
    ObjectClass::Program,
    ObjectClass::SetList,
    ObjectClass::Sample,
    ObjectClass::Piano,
    ObjectClass::Live,
    ObjectClass::Settings,
];

/// What the UI asks the instrument to do.
#[derive(Clone)]
pub enum DeviceCmd {
    /// One class end to end — counters and every bank — inside a single session,
    /// streaming a [`DeviceEvent::BankScanned`] as each bank lands.
    ScanClass {
        class: ObjectClass,
        /// Slots to ask each bank for.
        slots: u32,
        /// Ceiling on the walk, for a class whose size the counters cannot divide.
        banks: u32,
    },
    /// One `INFO` per slot of a single bank. What a mutation owes: only the bank it
    /// touched can have changed.
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
        /// The copy opens in a tab once it lands, which is what a double-click asks for.
        open: bool,
    },
    Put {
        class: ObjectClass,
        at: Location,
        name: String,
        bytes: Vec<u8>,
    },
    /// Every queued object of one class, written inside a single session.
    ///
    /// Each item still runs the whole read-back / delete / write / restore flow a lone
    /// [`DeviceCmd::Put`] runs; what is shared is the session around them. A refusal
    /// stops the batch where it stands.
    SendAll {
        class: ObjectClass,
        items: Vec<Outgoing>,
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

/// One object waiting to go back to the instrument.
#[derive(Clone)]
pub struct Outgoing {
    pub id: u64,
    pub at: Location,
    pub name: String,
    pub bytes: Vec<u8>,
}

/// How one operation is spoken about.
///
/// The activity log keeps the protocol line; the status strip gets sentences that name
/// places and things the way the panel does, and never a class number or a verb off the
/// wire.
#[derive(Clone)]
pub struct Words {
    pub log: String,
    pub doing: String,
    pub done: String,
    pub failed: String,
}

fn words(verbs: (&str, &str, &str), what: String, log: String) -> Words {
    Words {
        log,
        doing: format!("{} {what}…", verbs.0),
        done: format!("{} {what}", verbs.1),
        failed: format!("Could not {} {what}", verbs.2),
    }
}

const READING: (&str, &str, &str) = ("Reading", "Read", "read");
const COPYING: (&str, &str, &str) = ("Copying", "Copied", "copy");

impl DeviceCmd {
    /// What the log and the in-flight spinner call this operation.
    pub fn label(&self) -> String {
        match self {
            DeviceCmd::ScanClass { class, .. } => format!("scan {}", class.label()),
            DeviceCmd::ScanBank { bank, .. } => format!("scan bank {bank}"),
            DeviceCmd::SlotInfo { at, .. } => format!("info {}", shown(*at)),
            DeviceCmd::Deps { at, .. } => format!("deps {}", shown(*at)),
            DeviceCmd::Get { at, body, .. } => match body {
                true => format!("get {} (raw body)", shown(*at)),
                false => format!("get {}", shown(*at)),
            },
            DeviceCmd::Put { at, name, .. } => format!("put {name} -> {}", shown(*at)),
            DeviceCmd::SendAll { class, items } => {
                format!("put {} objects -> {}", items.len(), class.label())
            }
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

    /// The plain-words sentences the status strip shows for this operation.
    pub fn words(&self) -> Words {
        let log = self.label();
        match self {
            DeviceCmd::ScanClass { class, .. } => words(READING, folder(*class).to_string(), log),
            DeviceCmd::ScanBank { class, bank, .. } => {
                words(READING, format!("{} — bank {bank}", folder(*class)), log)
            }
            DeviceCmd::SlotInfo { class, at } => words(READING, place(*class, *at), log),
            DeviceCmd::Deps { class, at } => {
                words(READING, format!("what {} needs", place(*class, *at)), log)
            }
            DeviceCmd::Get { class, at, .. } => words(
                COPYING,
                format!("{} to this computer", place(*class, *at)),
                log,
            ),
            DeviceCmd::Put {
                class, at, name, ..
            } => words(
                ("Sending", "Sent", "send"),
                format!("“{name}” to {}", place(*class, *at)),
                log,
            ),
            DeviceCmd::SendAll { class, items } => words(
                ("Sending", "Sent", "send"),
                match items.len() {
                    1 => format!("1 sound to {}", folder(*class)),
                    n => format!("{n} sounds to {}", folder(*class)),
                },
                log,
            ),
            DeviceCmd::Move { class, from, to } => words(
                ("Moving", "Moved", "move"),
                format!("{} to {}", place(*class, *from), place(*class, *to)),
                log,
            ),
            DeviceCmd::Duplicate { class, from, to } => words(
                COPYING,
                format!("{} to {}", place(*class, *from), place(*class, *to)),
                log,
            ),
            DeviceCmd::Delete { class, at } => {
                words(("Deleting", "Deleted", "delete"), place(*class, *at), log)
            }
            DeviceCmd::Rename { class, at, name } => words(
                ("Renaming", "Renamed", "rename"),
                format!("{} to “{name}”", place(*class, *at)),
                log,
            ),
            DeviceCmd::Select { class, at } => words(
                ("Loading", "Loaded", "load"),
                format!("{} on the instrument", place(*class, *at)),
                log,
            ),
            DeviceCmd::Disconnect => words(
                ("Releasing", "Released", "release"),
                "the instrument".into(),
                log,
            ),
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
    /// A class's own counters, read at the head of its walk.
    ClassStatus {
        class: ObjectClass,
        status: Status,
        /// Banks the counters say to expect, where they divide.
        banks: Option<u32>,
    },
    BankScanned {
        class: ObjectClass,
        bank: u32,
        /// One entry per slot, `None` where the slot is vacant. Shorter than the bank
        /// asked for when the device said the class ends here.
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
        open: bool,
    },
    /// One object of a batch landed.
    Sent {
        id: u64,
        class: ObjectClass,
        at: Location,
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
    pub in_flight: Option<Words>,
    pub inventory: Vec<Status>,
    /// The background read of every class.
    pub scan: Scan,
    /// The slot this app last asked the instrument to load, per class.
    ///
    /// ⚠️ Only what **this app** selected. A selection made on the panel itself is
    /// invisible to the host — nothing on the wire reports it — so this is a record of
    /// our own commands, not of what the instrument is actually playing.
    selected: HashMap<u32, Location>,
    banks: HashMap<(u32, u32), Vec<Option<ProgramInfo>>>,
    pub detail: Detail,
}

impl DeviceState {
    pub fn connected(&self) -> bool {
        matches!(self.connection, Connection::Connected(_))
    }

    pub fn product(&self) -> Option<&str> {
        match &self.connection {
            Connection::Connected(card) => Some(card.product.as_str()),
            _ => None,
        }
    }

    /// A scanned bank's slots, or `None` if it has not been scanned.
    pub fn bank(&self, class: ObjectClass, bank: u32) -> Option<&[Option<ProgramInfo>]> {
        self.banks.get(&(class.to_raw(), bank)).map(Vec::as_slice)
    }

    /// The banks of a class that have been read, in order.
    pub fn banks_of(&self, class: ObjectClass) -> Vec<u32> {
        let mut banks: Vec<u32> = self
            .banks
            .keys()
            .filter(|(raw, _)| *raw == class.to_raw())
            .map(|(_, bank)| *bank)
            .collect();
        banks.sort_unstable();
        banks
    }

    /// What a slot holds, from the scan cache. `Some(None)` is a scanned empty slot.
    pub fn slot(&self, class: ObjectClass, at: Location) -> Option<Option<&ProgramInfo>> {
        let bank = self.bank(class, at.bank + 1)?;
        bank.get(at.slot as usize).map(Option::as_ref)
    }

    /// The first slot of `class` known to be vacant — where a duplicate lands when the
    /// user did not drag it anywhere.
    pub fn first_free(&self, class: ObjectClass) -> Option<Location> {
        self.banks_of(class).into_iter().find_map(|bank| {
            let slots = self.bank(class, bank)?;
            let slot = slots.iter().position(Option::is_none)?;
            Some(Location::from_user(bank, slot as u32 + 1))
        })
    }

    /// Drop one bank's cached names, because something just changed them.
    fn forget_bank(&mut self, class: ObjectClass, bank: u32) {
        self.banks.remove(&(class.to_raw(), bank));
    }

    fn forget_everything(&mut self) {
        self.banks.clear();
        self.inventory.clear();
        self.detail = Detail::default();
        self.scan.clear();
        self.selected.clear();
    }
}

/// ⚠️ A ceiling on a walk whose end the device alone decides. Nothing on the wire says a
/// class cannot hold more banks than this; the cap is here so an instrument that never
/// answers "out of range" cannot walk forever.
pub const MAX_BANKS: u32 = 32;

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

/// How full a folder is, in the terms the panel labels it with.
///
/// Slot counts only: the inventory also reports opaque block totals, and a class whose
/// items differ in size (pianos, samples) cannot be divided into slots at all — those
/// report their item count and nothing more.
pub fn holdings(class: ObjectClass, inventory: &[Status]) -> Option<String> {
    let status = inventory.iter().find(|status| status.class == class)?;
    Some(match status.slots() {
        Some(slots) => format!("{} of {slots} slots used", status.count),
        None => format!("{} items", status.count),
    })
}

/// Whether the browser offers to change a class at all.
///
/// ⚠️ A piano is a multi-megabyte library that the instrument builds its own index
/// over; the browser lists what is installed and offers nothing that would move it.
pub fn read_only(class: ObjectClass) -> bool {
    matches!(class, ObjectClass::Piano)
}

/// Whether this app will write into a class at all.
pub fn sendable(class: ObjectClass) -> bool {
    put_refusal(class).is_none() && !read_only(class)
}

/// Whether a class accepts a write.
///
/// ⚠️ A write is a delete followed by a write, and whether the live buffer or the
/// settings singleton survives a delete of its own class is unconfirmed on hardware.
/// Until it is, an edit of either stops at a file.
pub fn put_refusal(class: ObjectClass) -> Option<String> {
    match class {
        ObjectClass::Live | ObjectClass::Settings => Some(format!(
            "writing {} back over USB is unproven on hardware; save the object as a \
             file instead",
            class.label(),
        )),
        _ => None,
    }
}

/// Turn a device-supplied name into something a file picker can take later.
///
/// In the local list the name is the only record of what the bytes are, so it stays
/// readable: whitespace runs become one `-`, and only what a path cannot carry is
/// dropped.
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

pub struct Device {
    pub state: DeviceState,
    events: Receiver<DeviceEvent>,
    link: Link,
    /// What the user asked for. Always dispatched ahead of the background scan.
    pending: VecDeque<DeviceCmd>,
    /// The class the running command is walking, so a scan that fails is taken off the
    /// queue rather than left looking like it is still going.
    reading: Option<ObjectClass>,
    /// The banks the running mutation touches, to be read again once it finishes.
    rescan: Vec<(ObjectClass, u32)>,
    /// The loaded slots the running command overwrites. A batch can touch one per
    /// class, so this is a list rather than a single slot.
    reselect: Vec<(ObjectClass, Location)>,
}

impl Device {
    pub fn new(ctx: egui::Context) -> Device {
        let (sender, events) = std::sync::mpsc::channel();
        Device {
            state: DeviceState::default(),
            events,
            link: Link::new(ctx, sender),
            pending: VecDeque::new(),
            reading: None,
            rescan: Vec::new(),
            reselect: Vec::new(),
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
        log.say("Looking for an instrument…");
        self.link.connect();
    }

    pub fn disconnect(&mut self, log: &mut Log) {
        if !self.state.connected() {
            return;
        }
        log.say("Releasing the instrument…");
        self.pending.clear();
        self.state.scan.clear();
        self.link.disconnect();
    }

    /// Queue one command the user asked for. It runs ahead of the background read, and
    /// after whatever is already in flight — the protocol runs one transaction at a
    /// time.
    pub fn send(&mut self, cmd: DeviceCmd, log: &mut Log) {
        if !self.state.connected() {
            log.trouble("No instrument is attached.");
            return;
        }
        self.pending.push_back(cmd);
    }

    /// Walk `class` again, in one session.
    ///
    /// The names already cached stay up until each bank's replacement arrives: a walk is
    /// dozens of reads long, and emptying the folder for the length of one is worse than
    /// showing names that are about to be confirmed. What a mutation touched is dropped
    /// outright — see [`Device::dispatch`].
    pub fn read_class(&mut self, class: ObjectClass) {
        self.state.scan.start(class);
    }

    /// Start the next command if the instrument is free. Call once a frame, after the
    /// UI has had its say.
    pub fn pump(&mut self) {
        if !self.state.connected() || self.state.in_flight.is_some() {
            return;
        }
        if let Some(cmd) = self.pending.pop_front() {
            self.reading = None;
            return self.dispatch(cmd);
        }
        let Some(class) = self.state.scan.take() else {
            return;
        };
        self.reading = Some(class);
        self.dispatch(DeviceCmd::ScanClass {
            class,
            slots: slots_per_bank(class),
            banks: MAX_BANKS,
        });
    }

    fn dispatch(&mut self, cmd: DeviceCmd) {
        // A mutation makes the names the browser is showing wrong, so they are dropped
        // now rather than quietly kept — a stale name is what a confirmation would go on
        // to quote back at the operator. Only the banks it touches can have changed, so
        // only those are dropped and read again.
        self.rescan = match &cmd {
            DeviceCmd::Delete { class, at }
            | DeviceCmd::Rename { class, at, .. }
            | DeviceCmd::Put { class, at, .. } => vec![(*class, at.bank + 1)],
            DeviceCmd::Move { class, from, to } | DeviceCmd::Duplicate { class, from, to } => {
                vec![(*class, from.bank + 1), (*class, to.bank + 1)]
            }
            DeviceCmd::SendAll { class, items } => {
                let mut banks: Vec<(ObjectClass, u32)> =
                    items.iter().map(|item| (*class, item.at.bank + 1)).collect();
                banks.dedup();
                banks
            }
            _ => Vec::new(),
        };
        for (class, bank) in &self.rescan {
            self.state.forget_bank(*class, *bank);
        }
        // Writing into the slot the instrument has loaded leaves the panel playing the
        // buffer it read before the write; only a fresh SELECT reloads it. Confirmed on
        // hardware.
        let loaded = |state: &DeviceState, class: &ObjectClass, at: &Location| {
            state
                .selected
                .get(&class.to_raw())
                .filter(|held| *held == at)
                .map(|at| (*class, *at))
        };
        self.reselect = match &cmd {
            DeviceCmd::Put { class, at, .. } | DeviceCmd::Rename { class, at, .. } => {
                loaded(&self.state, class, at).into_iter().collect()
            }
            DeviceCmd::SendAll { class, items } => items
                .iter()
                .filter_map(|item| loaded(&self.state, class, &item.at))
                .collect(),
            _ => Vec::new(),
        };
        if let DeviceCmd::Select { class, at } = &cmd {
            self.state.selected.insert(class.to_raw(), *at);
        }
        self.state.in_flight = Some(cmd.words());
        self.link.send(cmd);
    }

    /// Fill in a bank as though the instrument had answered, for a headless render.
    #[cfg(test)]
    pub fn pretend_scanned(&mut self, class: ObjectClass, bank: u32, names: &[&str]) {
        use nord_usb::wire::ProgramInfo;

        self.state.connection = Connection::Connected(DeviceCard {
            product: "Electro 5".into(),
            manufacturer: None,
            vendor_id: 0x0ffc,
            product_id: 0,
            serial: None,
        });
        let slots = names
            .iter()
            .enumerate()
            .map(|(slot, name)| {
                // An empty name is a vacant slot, which is a row like any other.
                (!name.is_empty()).then(|| ProgramInfo {
                    location: Location::from_user(bank, slot as u32 + 1),
                    body_len: 121,
                    format: "ne5p".into(),
                    version: 4,
                    crc32: None,
                    name: (*name).to_string(),
                })
            })
            .collect();
        self.state.banks.insert((class.to_raw(), bank), slots);
    }

    /// Drain the worker's events into the cache, the local list and the tabs. Call once
    /// a frame.
    pub fn poll(&mut self, log: &mut Log, workspace: &mut Workspace, tabs: &mut Tabs) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                DeviceEvent::Connected(card) => {
                    log.info(format!(
                        "connected: {} ({:04x}:{:04x})",
                        card.product, card.vendor_id, card.product_id
                    ));
                    log.say(format!("{} is attached.", card.product));
                    self.state.connection = Connection::Connected(card);
                    self.state.forget_everything();
                    self.pending.clear();
                    // One walk per class, each its own session. Nothing is asked ahead
                    // of them: a class's counters are read at the head of its own walk.
                    for class in BROWSED {
                        self.read_class(class);
                    }
                }
                DeviceEvent::ConnectFailed(why) => {
                    log.error(why);
                    log.trouble("No instrument could be opened.");
                    self.state.connection = Connection::Disconnected;
                }
                DeviceEvent::Disconnected => {
                    log.say("The instrument was released.");
                    self.state.connection = Connection::Disconnected;
                    self.state.in_flight = None;
                    self.state.forget_everything();
                    self.pending.clear();
                    self.reading = None;
                }
                DeviceEvent::Started(what) => log.info(what),
                DeviceEvent::Finished => {
                    if let Some(class) = self.reading.take() {
                        self.state.scan.finished(class);
                    }
                    // The panel is still playing what it read before the write, so it is
                    // asked to load the slot again. `select` is read-only.
                    for (class, at) in std::mem::take(&mut self.reselect) {
                        self.pending.push_back(DeviceCmd::Select { class, at });
                    }
                    for (class, bank) in std::mem::take(&mut self.rescan) {
                        self.pending.push_back(DeviceCmd::ScanBank {
                            class,
                            bank,
                            slots: slots_per_bank(class),
                        });
                    }
                    self.state.in_flight = None;
                }
                DeviceEvent::ClassStatus {
                    class,
                    status,
                    banks,
                } => {
                    self.state.inventory.retain(|held| held.class != class);
                    self.state.inventory.push(status);
                    self.state.scan.expect(class, banks);
                }
                DeviceEvent::BankScanned { class, bank, slots } => {
                    if !slots.is_empty() {
                        self.state.banks.insert((class.to_raw(), bank), slots);
                    }
                    self.state.scan.bank(class, bank);
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
                    open,
                } => {
                    let id = workspace.ingest(name, origin, bytes, log);
                    if open {
                        tabs.open(id, workspace);
                    }
                }
                DeviceEvent::Rescued { at, name, bytes } => {
                    log.error(format!(
                        "{} could not be restored; its bytes are in the local list as {name}",
                        shown(at)
                    ));
                    log.trouble(format!(
                        "{} is empty — what was in it is on this computer as “{name}”.",
                        shown(at)
                    ));
                    workspace.ingest(name, Origin::Rescued { at }, bytes, log);
                }
                // One of a batch landed, so it is no longer owed.
                DeviceEvent::Sent { id, .. } => workspace.mark_pending(id, false),
                DeviceEvent::Note(text) => log.info(text),
                DeviceEvent::OpOk(text) => {
                    log.info(text);
                    if let Some(words) = &self.state.in_flight {
                        log.say(format!("{}.", words.done));
                    }
                }
                DeviceEvent::OpFailed(text) => {
                    log.error(text);
                    match &self.state.in_flight {
                        Some(words) => {
                            let failed = words.failed.clone();
                            log.trouble(format!("{failed}. The details are below."));
                        }
                        None => log.trouble("Something went wrong. The details are below."),
                    }
                }
                // Something changed outside our session, so every cached name may now be
                // wrong. They are dropped and read again rather than flagged: a name the
                // user is about to drag is one a dialog would go on to quote back.
                DeviceEvent::InstrumentChanged => {
                    log.warn("the instrument changed under us — every cached name is dropped");
                    log.say("Something changed on the instrument. Reading it again…");
                    for class in BROWSED {
                        self.read_class(class);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The label is the only description the local list will ever have of a slot's
    /// contents, so it survives into the name rather than being reduced to something
    /// opaque.
    #[test]
    fn a_name_keeps_the_words_it_was_given() {
        assert_eq!(stem("split point C4"), "split-point-C4");
        assert_eq!(stem("  transpose +1  "), "transpose-+1");
        assert_eq!(stem("organ vol 5 -> 6"), "organ-vol-5-6");
    }

    /// The name reaches a file picker later, so nothing in it may be a path.
    #[test]
    fn a_name_cannot_become_a_path() {
        assert_eq!(stem("../../etc/passwd"), "etc-passwd");
        assert_eq!(stem("rotary:fast"), "rotary-fast");
        assert_eq!(stem(".hidden"), "hidden");
    }

    /// Nothing usable left is reported as nothing, for the caller to refuse or
    /// substitute — never silently turned into a default.
    #[test]
    fn a_name_with_nothing_in_it_comes_back_empty() {
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
    }

    /// Pianos are listed and never altered.
    #[test]
    fn only_pianos_are_read_only() {
        assert!(read_only(ObjectClass::Piano));
        for class in BROWSED.iter().filter(|c| **c != ObjectClass::Piano) {
            assert!(!read_only(*class), "{}", folder(*class));
        }
    }

    /// The status strip names places and things; the protocol line keeps the verbs.
    #[test]
    fn the_status_sentences_never_quote_the_protocol() {
        let cmd = DeviceCmd::Put {
            class: ObjectClass::Program,
            at: Location { bank: 6, slot: 3 },
            name: "Africa Split".into(),
            bytes: Vec::new(),
        };
        let words = cmd.words();
        assert_eq!(words.doing, "Sending “Africa Split” to Programs 7:4…");
        assert_eq!(words.done, "Sent “Africa Split” to Programs 7:4");
        assert_eq!(
            words.failed,
            "Could not send “Africa Split” to Programs 7:4"
        );
        assert_eq!(words.log, "put Africa Split -> 7:4");
    }

    /// The folder name is what the panel calls the thing, never the class number.
    #[test]
    fn a_place_reads_as_a_folder_and_a_slot() {
        let at = Location { bank: 0, slot: 0 };
        assert_eq!(place(ObjectClass::SetList, at), "Set lists 1:1");
        assert_eq!(folder(ObjectClass::Unknown(9)), "Other");
    }
}
