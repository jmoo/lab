//! This computer: the assets held in memory, and the ways bytes get in and out of them.
//!
//! An entity is bytes first and a decode second — a file that does not parse still
//! gets a row, with its error and its raw body still exportable, because reporting a
//! bad file is the point of opening it.
//!
//! The list draws itself in [`crate::browser`]; what lives here is the model and the
//! file dialogs.

use std::sync::mpsc::{Receiver, Sender};

use eframe::egui;
use nord_format::cbin::{Generation, Header};
use nord_format::formats::ne5;
use nord_format::{Entity, Live, Program, Settings};
use nord_usb::{Location, ObjectClass};

use crate::log::Log;

/// Where an entity came from.
#[derive(Clone)]
pub enum Origin {
    File(String),
    Device {
        class: ObjectClass,
        at: Location,
    },
    Fresh,
    /// The occupant of a slot that a failed write could not put back.
    Rescued {
        at: Location,
    },
}

impl Origin {
    pub fn label(&self) -> String {
        match self {
            Origin::File(name) => format!("Opened from {name}"),
            Origin::Device { class, at } => {
                format!("Copied from {}", crate::strings::place(*class, *at))
            }
            Origin::Fresh => "New, not saved anywhere yet".into(),
            Origin::Rescued { at } => format!("Rescued from {}", crate::strings::shown(*at)),
        }
    }

    /// The slot this came off, for the tab header's way back.
    pub fn slot(&self) -> Option<(ObjectClass, Location)> {
        match self {
            Origin::Device { class, at } => Some((*class, *at)),
            _ => None,
        }
    }
}

/// Whether re-encoding a decode reproduced the bytes it came from.
#[derive(Clone)]
pub enum VerifyState {
    Ok,
    /// Offset of the first byte that came back different.
    Differs {
        at: usize,
    },
    /// Re-encoding refused.
    Failed(String),
    /// Nothing to check, and why.
    NotApplicable(&'static str),
}

impl VerifyState {
    pub fn badge(&self) -> &'static str {
        match self {
            VerifyState::Ok => "ok",
            VerifyState::Differs { .. } => "differs",
            VerifyState::Failed(_) => "failed",
            VerifyState::NotApplicable(_) => "n/a",
        }
    }

    pub fn detail(&self) -> String {
        match self {
            VerifyState::Ok => "re-encoded byte-for-byte".into(),
            VerifyState::Differs { at } => format!("first difference at byte {at:#06x}"),
            VerifyState::Failed(why) => why.clone(),
            VerifyState::NotApplicable(why) => (*why).to_string(),
        }
    }

    pub fn color(&self, visuals: &egui::Visuals) -> egui::Color32 {
        match self {
            VerifyState::Ok => crate::app::good(visuals),
            VerifyState::Differs { .. } | VerifyState::Failed(_) => crate::app::bad(visuals),
            VerifyState::NotApplicable(_) => visuals.weak_text_color(),
        }
    }
}

/// The container facts, read once at ingest.
///
/// ⚠️ Reading them streams the whole file to check the checksum, so it happens on the
/// way in and never per frame — a piano library is hundreds of megabytes.
#[derive(Clone)]
pub struct Container {
    pub header: Header,
    pub body_len: u64,
    pub checksum_ok: bool,
    /// `crc32:` or `crc16:` — the two generations keep it in different places.
    pub checksum_label: &'static str,
    pub checksum: String,
}

impl Container {
    fn read(bytes: &[u8]) -> Option<Container> {
        let info = nord_format::cbin::inspect(&mut std::io::Cursor::new(bytes)).ok()?;
        // The one header fact the parsed `Header` does not carry: a type-1 file holds
        // a crc32 over its body at 0x18, a type-0 file a crc16 over the whole file in
        // its last two bytes.
        let (checksum_label, checksum) = match info.header.generation {
            Generation::V0 => {
                let tail = bytes.get(bytes.len().checked_sub(2)?..)?;
                let crc = u16::from_le_bytes(tail.try_into().ok()?);
                ("crc16:", format!("{crc:#06x}"))
            }
            Generation::V1 => {
                let crc = u32::from_le_bytes(bytes.get(0x18..0x1c)?.try_into().ok()?);
                ("crc32:", format!("{crc:#010x}"))
            }
        };
        Some(Container {
            header: info.header,
            body_len: info.body_len,
            checksum_ok: info.checksum_ok,
            checksum_label,
            checksum,
        })
    }

    pub fn tag(&self) -> String {
        String::from_utf8_lossy(&self.header.tag).into_owned()
    }
}

/// One object held in memory: its bytes, what they decode to, and how they got here.
pub struct LocalEntity {
    /// Stable across reordering, so a selection survives a removal.
    pub id: u64,
    pub name: String,
    pub origin: Origin,
    pub bytes: Vec<u8>,
    pub entity: Option<Entity>,
    pub parse_error: Option<String>,
    pub container: Option<Container>,
    pub verify: VerifyState,
    pub dirty: bool,
    /// Owed back to the slot it came from, waiting on a Send.
    pub pending: bool,
}

impl LocalEntity {
    fn new(id: u64, name: String, origin: Origin, bytes: Vec<u8>) -> LocalEntity {
        let container = Container::read(&bytes);
        let (entity, parse_error) =
            match nord_format::from_stream(&mut std::io::Cursor::new(&bytes)) {
                Ok(entity) => (Some(entity), None),
                Err(e) => (None, Some(e.to_string())),
            };
        let verify = match &entity {
            Some(entity) => verify(entity, &bytes),
            None => VerifyState::NotApplicable("the file did not decode"),
        };
        LocalEntity {
            id,
            name,
            origin,
            bytes,
            entity,
            parse_error,
            container,
            verify,
            dirty: false,
            pending: false,
        }
    }

    /// The format tag, from the decode where there is one and the container otherwise.
    pub fn tag(&self) -> String {
        match (&self.entity, &self.container) {
            (Some(entity), _) => entity.identity().format.to_string(),
            (None, Some(container)) => container.tag(),
            (None, None) => "?".into(),
        }
    }

    /// The bytes the wire would carry: the file with its container stripped.
    ///
    /// The counterpart of `nord … get --body` pointed at a file. `None` for anything
    /// that is not a CBIN container.
    pub fn raw_body(&self) -> Option<Vec<u8>> {
        nord_usb::envelope::unwrap(&self.bytes)
            .ok()
            .map(|read| read.body.0)
    }
}

/// Re-encode and compare — the check `nord verify` runs on a file.
fn verify(entity: &Entity, bytes: &[u8]) -> VerifyState {
    if matches!(entity, Entity::Bundle(_)) {
        return VerifyState::NotApplicable("a bundle is an archive; it does not re-encode");
    }
    let out = match nord_format::to_bytes(entity) {
        Ok(out) => out,
        Err(e) => return VerifyState::Failed(e.to_string()),
    };
    match out.iter().zip(bytes).position(|(a, b)| a != b) {
        Some(at) => VerifyState::Differs { at },
        None if out.len() == bytes.len() => VerifyState::Ok,
        // A shared prefix and a different length: they part at the end of the shorter.
        None => VerifyState::Differs {
            at: out.len().min(bytes.len()),
        },
    }
}

/// The fresh defaults the New menu offers — the objects `nord … edit` starts from
/// when it is given no target.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Fresh {
    Program,
    Live,
    Settings,
}

impl Fresh {
    pub const ALL: [Fresh; 3] = [Fresh::Program, Fresh::Live, Fresh::Settings];

    pub fn label(self) -> &'static str {
        match self {
            Fresh::Program => "program",
            Fresh::Live => "live",
            Fresh::Settings => "settings",
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Fresh::Program => ne5::program::FORMAT,
            Fresh::Live => ne5::live::FORMAT,
            Fresh::Settings => ne5::settings::FORMAT,
        }
    }

    fn bytes(self) -> Result<Vec<u8>, String> {
        let entity = match self {
            Fresh::Program => Entity::Program(Program::Electro5(ne5::program::new(
                (0, 0).try_into().map_err(|e| format!("{e}"))?,
            ))),
            Fresh::Live => Entity::Live(Live::Electro5(ne5::live::new(
                (0, 0).try_into().map_err(|e| format!("{e}"))?,
            ))),
            Fresh::Settings => Entity::Settings(Settings::Electro5(ne5::settings::new())),
        };
        nord_format::to_bytes(&entity).map_err(|e| e.to_string())
    }
}

/// One asset as a store holds it.
pub struct Saved {
    pub id: u64,
    pub name: String,
    pub origin: Origin,
    pub bytes: Vec<u8>,
}

/// What a background task hands back to the UI thread.
enum Incoming {
    Opened { name: String, bytes: Vec<u8> },
    Note(String),
    Failed(String),
}

/// Which bytes a save writes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExportWhat {
    File,
    Body,
}

pub struct Workspace {
    entities: Vec<LocalEntity>,
    selected: Option<u64>,
    next_id: u64,
    /// Bumped by every change to the list, so the shell can tell when the store is
    /// behind without comparing every asset's bytes.
    revision: u64,
    ctx: egui::Context,
    tx: Sender<Incoming>,
    rx: Receiver<Incoming>,
}

impl Workspace {
    pub fn new(ctx: egui::Context) -> Workspace {
        let (tx, rx) = std::sync::mpsc::channel();
        Workspace {
            entities: Vec::new(),
            selected: None,
            next_id: 1,
            revision: 0,
            ctx,
            tx,
            rx,
        }
    }

    pub fn selected(&self) -> Option<&LocalEntity> {
        let id = self.selected?;
        self.entities.iter().find(|e| e.id == id)
    }

    /// Point the document view at one entity. The browser's own selection is separate:
    /// clicking a row in the sidebar does not change what the open tab is showing.
    pub fn select(&mut self, id: Option<u64>) {
        self.selected = id;
    }

    /// Counts changes to the list, not to any one asset.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn entities(&self) -> &[LocalEntity] {
        &self.entities
    }

    pub fn get(&self, id: u64) -> Option<&LocalEntity> {
        self.entities.iter().find(|e| e.id == id)
    }

    /// Mark an asset as owed back to the slot it came from, or paid.
    pub fn mark_pending(&mut self, id: u64, pending: bool) {
        if let Some(entity) = self.entities.iter_mut().find(|e| e.id == id) {
            if entity.pending != pending {
                entity.pending = pending;
                self.revision += 1;
            }
        }
    }

    /// Everything waiting to go back to the instrument, in the order it was opened.
    pub fn pending(&self) -> Vec<&LocalEntity> {
        self.entities.iter().filter(|e| e.pending).collect()
    }

    /// Rename an asset held here. Nothing leaves this computer.
    pub fn rename(&mut self, id: u64, name: String) {
        if let Some(entity) = self.entities.iter_mut().find(|e| e.id == id) {
            entity.name = name;
            self.revision += 1;
        }
    }

    /// Decode `bytes`, badge them, and add the row. Every way in — drop, picker,
    /// fresh default, and later a device read — lands here.
    pub fn ingest(&mut self, name: String, origin: Origin, bytes: Vec<u8>, log: &mut Log) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let entity = LocalEntity::new(id, name, origin, bytes);
        match (&entity.parse_error, &entity.verify) {
            (Some(e), _) => {
                log.error(format!("{}: {e}", entity.name));
                log.trouble(format!(
                    "“{}” is not a file this app understands.",
                    entity.name
                ));
            }
            (None, VerifyState::Ok) => {
                log.info(format!(
                    "{}: {} ({} bytes), verified",
                    entity.name,
                    entity.tag(),
                    entity.bytes.len(),
                ));
                log.say(format!("“{}” is on this computer.", entity.name));
            }
            (None, state) => {
                log.warn(format!(
                    "{}: {} — verify {}: {}",
                    entity.name,
                    entity.tag(),
                    state.badge(),
                    state.detail(),
                ));
                log.say(format!(
                    "“{}” opened, but it does not re-save byte for byte.",
                    entity.name
                ));
            }
        }
        self.entities.push(entity);
        self.selected = Some(id);
        self.revision += 1;
        id
    }

    /// Drain whatever the pickers finished with. Call once per frame.
    pub fn poll(&mut self, log: &mut Log) {
        while let Ok(message) = self.rx.try_recv() {
            match message {
                Incoming::Opened { name, bytes } => {
                    self.ingest(name.clone(), Origin::File(name), bytes, log);
                }
                Incoming::Note(text) => log.say(text),
                Incoming::Failed(text) => log.trouble(text),
            }
        }
    }

    pub fn open_dialog(&self) {
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        spawn(async move {
            let picked = rfd::AsyncFileDialog::new()
                .set_title("Open Nord files")
                .pick_files()
                .await;
            for handle in picked.unwrap_or_default() {
                let bytes = handle.read().await;
                let _ = tx.send(Incoming::Opened {
                    name: handle.file_name(),
                    bytes,
                });
            }
            ctx.request_repaint();
        });
    }

    /// The filename an export suggests.
    ///
    /// ⚠️ The one place a name becomes a filename. A file stores no name of its own —
    /// the name is this app's metadata, carried since the moment the bytes arrived — so
    /// nothing may re-derive it from the bytes here or anywhere else.
    pub fn export_name(&self, id: u64, what: ExportWhat) -> Option<String> {
        let entity = self.get(id)?;
        Some(match what {
            ExportWhat::File => entity.name.clone(),
            ExportWhat::Body => format!("{}.body", entity.name),
        })
    }

    pub fn export(&self, id: u64, what: ExportWhat) {
        let Some(entity) = self.entities.iter().find(|e| e.id == id) else {
            return;
        };
        let name = match self.export_name(id, what) {
            Some(name) => name,
            None => return,
        };
        let bytes = match what {
            ExportWhat::File => Some(entity.bytes.clone()),
            ExportWhat::Body => entity.raw_body(),
        };
        let Some(bytes) = bytes else {
            let _ = self.tx.send(Incoming::Failed(format!(
                "{}: not a CBIN container, so there is no header to strip",
                entity.name,
            )));
            return;
        };
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        spawn(async move {
            let _ = tx.send(save(name, bytes).await);
            ctx.request_repaint();
        });
    }

    /// Put back the bytes a tab opened with. The asset is unchanged again, so the dirty
    /// mark goes with them.
    pub fn restore_bytes(&mut self, id: u64, bytes: Vec<u8>, log: &mut Log) {
        let Some(entity) = self.entities.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if entity.bytes == bytes {
            return;
        }
        *entity = LocalEntity::new(id, entity.name.clone(), entity.origin.clone(), bytes);
        self.revision += 1;
        log.say(format!("“{}” is back as it was opened.", entity.name));
    }

    /// Swap in re-encoded bytes, keeping the entity's identity and marking it edited.
    ///
    /// The decode and the verify are re-run: an editor's output is bytes like any other,
    /// and it earns its badge the same way a file off disk does.
    pub fn replace_bytes(&mut self, id: u64, bytes: Vec<u8>, log: &mut Log) {
        let Some(entity) = self.entities.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let replaced = LocalEntity::new(id, entity.name.clone(), entity.origin.clone(), bytes);
        let verify = replaced.verify.clone();
        *entity = LocalEntity {
            dirty: true,
            pending: entity.pending,
            ..replaced
        };
        self.revision += 1;
        if let VerifyState::Ok = verify {
            return;
        }
        log.warn(format!(
            "after editing, verify {}: {}",
            verify.badge(),
            verify.detail()
        ));
    }

    pub fn duplicate(&mut self, id: u64, log: &mut Log) -> Option<u64> {
        let source = self.entities.iter().find(|e| e.id == id)?;
        let name = format!("{} copy", source.name);
        let (origin, bytes) = (source.origin.clone(), source.bytes.clone());
        Some(self.ingest(name, origin, bytes, log))
    }

    pub fn remove(&mut self, id: u64, log: &mut Log) {
        let Some(at) = self.entities.iter().position(|e| e.id == id) else {
            return;
        };
        let gone = self.entities.remove(at);
        self.revision += 1;
        if self.selected == Some(id) {
            self.selected = self.entities.last().map(|e| e.id);
        }
        log.say(format!("Removed “{}” from this computer.", gone.name));
    }

    /// The next id a new asset would take, for the store to carry over.
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// Put back what a previous session held.
    ///
    /// Every asset is decoded and re-checked on the way in: bytes out of a store have
    /// been sitting somewhere this app does not control and get no more trust than bytes
    /// off a disk.
    pub fn restore(&mut self, saved: Vec<Saved>, next_id: Option<u64>, log: &mut Log) {
        for Saved {
            id,
            name,
            origin,
            bytes,
        } in saved
        {
            let entity = LocalEntity::new(id, name, origin, bytes);
            if let Some(e) = &entity.parse_error {
                log.warn(format!("{}: {e}", entity.name));
            }
            self.next_id = self.next_id.max(id + 1);
            self.entities.push(entity);
        }
        if let Some(next) = next_id {
            self.next_id = self.next_id.max(next);
        }
        self.selected = self.entities.last().map(|e| e.id);
        self.revision += 1;
    }

    /// Make one of the fresh defaults and add it to the list.
    pub fn create(&mut self, kind: Fresh, log: &mut Log) -> Option<u64> {
        match kind.bytes() {
            Ok(bytes) => {
                let name = format!("untitled.{}", kind.tag());
                Some(self.ingest(name, Origin::Fresh, bytes, log))
            }
            Err(e) => {
                log.error(format!("new {}: {e}", kind.label()));
                log.trouble(format!("Could not make a new {}.", kind.label()));
                None
            }
        }
    }
}

/// Hand `bytes` to the user under `name`, however this target saves a file.
#[cfg(not(target_arch = "wasm32"))]
async fn save(name: String, bytes: Vec<u8>) -> Incoming {
    let Some(handle) = rfd::AsyncFileDialog::new()
        .set_file_name(&name)
        .save_file()
        .await
    else {
        return Incoming::Note(format!("{name}: save cancelled"));
    };
    match handle.write(&bytes).await {
        Ok(()) => Incoming::Note(format!(
            "wrote {} ({} bytes)",
            handle.file_name(),
            bytes.len(),
        )),
        Err(e) => Incoming::Failed(format!("{name}: {e}")),
    }
}

/// The browser has no save picker: the bytes become a Blob behind an object URL, and
/// a synthetic anchor click hands that to the downloader.
#[cfg(target_arch = "wasm32")]
async fn save(name: String, bytes: Vec<u8>) -> Incoming {
    match download(&name, &bytes) {
        Ok(()) => Incoming::Note(format!("downloaded {name} ({} bytes)", bytes.len())),
        Err(e) => Incoming::Failed(format!("{name}: {e:?}")),
    }
}

#[cfg(target_arch = "wasm32")]
fn download(name: &str, bytes: &[u8]) -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast as _;
    use wasm_bindgen::JsValue;

    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| JsValue::from_str("no document"))?;

    // `Uint8Array::from` copies into the JS heap, so the Blob does not alias Rust
    // memory that is about to be freed.
    let parts = js_sys::Array::new();
    parts.push(&js_sys::Uint8Array::from(bytes).into());
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("application/octet-stream");
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)?;

    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let anchor: web_sys::HtmlAnchorElement = document.create_element("a")?.unchecked_into();
    anchor.set_href(&url);
    anchor.set_download(name);
    anchor.click();
    web_sys::Url::revoke_object_url(&url)?;
    Ok(())
}

/// Run a task that outlives the frame that started it.
///
/// ⚠️ wasm has one thread and cannot block: the future has to go to the microtask
/// queue, never to a thread, or the picker never resolves.
#[cfg(not(target_arch = "wasm32"))]
fn spawn<F: std::future::Future<Output = ()> + Send + 'static>(future: F) {
    std::thread::spawn(move || nord_usb::block_on(future));
}

#[cfg(target_arch = "wasm32")]
fn spawn<F: std::future::Future<Output = ()> + 'static>(future: F) {
    wasm_bindgen_futures::spawn_local(future);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ingest(name: &str, bytes: Vec<u8>) -> LocalEntity {
        LocalEntity::new(1, name.into(), Origin::Fresh, bytes)
    }

    #[test]
    fn a_fresh_program_decodes_and_verifies() {
        let entity = ingest("untitled.ne5p", Fresh::Program.bytes().unwrap());
        assert!(entity.parse_error.is_none());
        assert_eq!(entity.tag(), "ne5p");
        assert!(
            matches!(entity.verify, VerifyState::Ok),
            "{}",
            entity.verify.detail()
        );

        let container = entity.container.expect("a fresh program is a CBIN file");
        assert!(container.checksum_ok);
        assert_eq!(container.header.generation, Generation::V1);
        assert_eq!(container.body_len, ne5::program::BODY_LEN as u64);
        assert_eq!(container.checksum_label, "crc32:");
    }

    /// Each fresh default carries its own tag, and each one round-trips.
    #[test]
    fn every_fresh_default_round_trips_under_its_own_tag() {
        for kind in Fresh::ALL {
            let entity = ingest("untitled", kind.bytes().unwrap());
            assert_eq!(entity.tag(), kind.tag());
            assert!(matches!(entity.verify, VerifyState::Ok), "{}", kind.label());
        }
    }

    /// The raw-body export is the file with its container stripped — for a type-1
    /// file, everything from `body_start` on.
    #[test]
    fn the_raw_body_export_drops_the_container_header() {
        let entity = ingest("untitled.ne5p", Fresh::Program.bytes().unwrap());
        let body = entity.raw_body().expect("a CBIN file has a body");
        assert_eq!(body.len(), ne5::program::BODY_LEN);
        assert_eq!(
            body.as_slice(),
            &entity.bytes[Generation::V1.body_start() as usize..],
        );
    }

    /// A file that does not decode is still a row: the error is the report.
    #[test]
    fn bytes_that_do_not_decode_are_kept_with_their_error() {
        let entity = ingest("junk.bin", b"not a nord file at all".to_vec());
        assert!(entity.entity.is_none());
        assert!(entity.parse_error.is_some());
        assert!(entity.container.is_none());
        assert!(matches!(entity.verify, VerifyState::NotApplicable(_)));
        assert_eq!(entity.tag(), "?");
    }

    /// The name is this app's metadata and the only record of what an object is — a
    /// file stores none. It has to survive the whole way: off the instrument, into a
    /// tab, through an edit, out to a filename.
    #[test]
    fn a_name_survives_being_fetched_opened_edited_and_exported() {
        use nord_usb::{Location, ObjectClass};

        let ctx = egui::Context::default();
        let mut workspace = Workspace::new(ctx);
        let mut log = Log::default();
        let at = Location { bank: 6, slot: 3 };

        // As a read off the instrument arrives: the device supplies the name, nothing
        // else ever does.
        let id = workspace.ingest(
            "Africa-Split.ne5p".into(),
            Origin::Device {
                class: ObjectClass::Program,
                at,
            },
            Fresh::Program.bytes().unwrap(),
            &mut log,
        );
        assert_eq!(workspace.get(id).unwrap().name, "Africa-Split.ne5p");

        // An edit: new bytes, same name.
        let bytes = workspace.get(id).unwrap().bytes.clone();
        let (_, edited) =
            crate::fields::apply(&bytes, &[("center_panel.gain".into(), "96".into())]).unwrap();
        workspace.replace_bytes(id, edited, &mut log);
        assert_eq!(workspace.get(id).unwrap().name, "Africa-Split.ne5p");
        assert!(workspace.get(id).unwrap().dirty);

        // Reverting is not renaming either.
        workspace.restore_bytes(id, bytes, &mut log);
        assert_eq!(workspace.get(id).unwrap().name, "Africa-Split.ne5p");

        // And the filename an export offers is that same name, not one worked back out
        // of the bytes.
        assert_eq!(
            workspace.export_name(id, ExportWhat::File).as_deref(),
            Some("Africa-Split.ne5p")
        );
        assert_eq!(
            workspace.export_name(id, ExportWhat::Body).as_deref(),
            Some("Africa-Split.ne5p.body")
        );
    }

    /// What a previous session held comes back decoded and checked, keeping its name,
    /// its origin and its id.
    #[test]
    fn a_restored_asset_keeps_what_it_was() {
        let ctx = egui::Context::default();
        let mut workspace = Workspace::new(ctx);
        let mut log = Log::default();
        workspace.restore(
            vec![Saved {
                id: 9,
                name: "Africa-Split.ne5p".into(),
                origin: Origin::Fresh,
                bytes: Fresh::Program.bytes().unwrap(),
            }],
            Some(10),
            &mut log,
        );
        let entity = workspace.get(9).expect("restored under its own id");
        assert_eq!(entity.name, "Africa-Split.ne5p");
        assert!(matches!(entity.verify, VerifyState::Ok));
        assert!(!entity.dirty);
        // A new asset cannot land on an id something restored is already using.
        let fresh = workspace.create(Fresh::Live, &mut log).unwrap();
        assert!(fresh >= 10);
    }

    /// A flipped body byte is reported rather than absorbed: the container's own
    /// checksum no longer matches, and the decode says so.
    #[test]
    fn a_tampered_body_byte_is_reported() {
        let mut bytes = Fresh::Program.bytes().unwrap();
        let at = Generation::V1.body_start() as usize + 0x30;
        bytes[at] ^= 0xff;
        let entity = ingest("tampered.ne5p", bytes);
        assert!(entity.parse_error.is_some());
        assert!(!entity.container.expect("still a CBIN file").checksum_ok);
    }
}
