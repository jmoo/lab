//! The workspace pane: local entities and the ways bytes get in and out of it.
//!
//! An entity is bytes first and a decode second — a file that does not parse still
//! gets a row, with its error and its raw body still exportable, because reporting a
//! bad file is the point of opening it.

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
    Sweep {
        label: String,
    },
}

impl Origin {
    pub fn label(&self) -> String {
        match self {
            Origin::File(name) => format!("file {name}"),
            Origin::Device { class, at } => format!("{} {}", class.label(), shown(*at)),
            Origin::Fresh => "new".into(),
            Origin::Rescued { at } => format!("rescued {}", shown(*at)),
            Origin::Sweep { label } => format!("sweep {label:?}"),
        }
    }
}

/// One-indexed `BANK:SLOT`, the way the instrument and Nord Sound Manager label a
/// location.
pub fn shown(at: Location) -> String {
    format!("{}:{}", at.bank + 1, at.slot + 1)
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

    pub fn color(&self) -> egui::Color32 {
        match self {
            VerifyState::Ok => crate::app::GOOD,
            VerifyState::Differs { .. } | VerifyState::Failed(_) => crate::app::BAD,
            VerifyState::NotApplicable(_) => egui::Color32::GRAY,
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

/// What a background task hands back to the UI thread.
enum Incoming {
    Opened { name: String, bytes: Vec<u8> },
    Note(String),
    Failed(String),
}

/// Which bytes an export writes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ExportWhat {
    File,
    Body,
}

pub struct Workspace {
    entities: Vec<LocalEntity>,
    selected: Option<u64>,
    next_id: u64,
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
            ctx,
            tx,
            rx,
        }
    }

    pub fn selected(&self) -> Option<&LocalEntity> {
        let id = self.selected?;
        self.entities.iter().find(|e| e.id == id)
    }

    /// Decode `bytes`, badge them, and add the row. Every way in — drop, picker,
    /// fresh default, and later a device read — lands here.
    pub fn ingest(&mut self, name: String, origin: Origin, bytes: Vec<u8>, log: &mut Log) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let entity = LocalEntity::new(id, name, origin, bytes);
        match (&entity.parse_error, &entity.verify) {
            (Some(e), _) => log.error(format!("{}: {e}", entity.name)),
            (None, VerifyState::Ok) => log.info(format!(
                "{}: {} ({} bytes), verified",
                entity.name,
                entity.tag(),
                entity.bytes.len(),
            )),
            (None, state) => log.warn(format!(
                "{}: {} — verify {}: {}",
                entity.name,
                entity.tag(),
                state.badge(),
                state.detail(),
            )),
        }
        self.entities.push(entity);
        self.selected = Some(id);
        id
    }

    /// Drain whatever the pickers finished with. Call once per frame.
    pub fn poll(&mut self, log: &mut Log) {
        while let Ok(message) = self.rx.try_recv() {
            match message {
                Incoming::Opened { name, bytes } => {
                    self.ingest(name.clone(), Origin::File(name), bytes, log);
                }
                Incoming::Note(text) => log.info(text),
                Incoming::Failed(text) => log.error(text),
            }
        }
    }

    fn open_dialog(&self) {
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

    fn export(&self, id: u64, what: ExportWhat) {
        let Some(entity) = self.entities.iter().find(|e| e.id == id) else {
            return;
        };
        let (name, bytes) = match what {
            ExportWhat::File => (entity.name.clone(), Some(entity.bytes.clone())),
            ExportWhat::Body => (format!("{}.body", entity.name), entity.raw_body()),
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
            ..replaced
        };
        if let VerifyState::Ok = verify {
            return;
        }
        log.warn(format!(
            "after editing, verify {}: {}",
            verify.badge(),
            verify.detail()
        ));
    }

    fn duplicate(&mut self, id: u64, log: &mut Log) {
        let Some(source) = self.entities.iter().find(|e| e.id == id) else {
            return;
        };
        let name = format!("{} copy", source.name);
        let (origin, bytes) = (source.origin.clone(), source.bytes.clone());
        self.ingest(name, origin, bytes, log);
    }

    fn remove(&mut self, id: u64, log: &mut Log) {
        let Some(at) = self.entities.iter().position(|e| e.id == id) else {
            return;
        };
        let gone = self.entities.remove(at);
        if self.selected == Some(id) {
            self.selected = self.entities.last().map(|e| e.id);
        }
        log.info(format!("removed {}", gone.name));
    }

    /// The workspace pane. Returns the id of a freshly created default, which the
    /// caller opens in the editor — a new object exists to be edited.
    pub fn ui(&mut self, ui: &mut egui::Ui, log: &mut Log) -> Option<u64> {
        let mut created = None;
        let mut fresh = None;
        ui.horizontal_wrapped(|ui| {
            if ui.button("Open…").clicked() {
                self.open_dialog();
            }
            ui.menu_button("New ▾", |ui| {
                for kind in Fresh::ALL {
                    if ui.button(kind.label()).clicked() {
                        fresh = Some(kind);
                        ui.close();
                    }
                }
            });
        });
        if let Some(kind) = fresh {
            match kind.bytes() {
                Ok(bytes) => {
                    let name = format!("untitled.{}", kind.tag());
                    created = Some(self.ingest(name, Origin::Fresh, bytes, log));
                }
                Err(e) => log.error(format!("new {}: {e}", kind.label())),
            }
        }
        ui.separator();

        if self.entities.is_empty() {
            ui.label(
                egui::RichText::new("Drop Nord files here, or use Open…")
                    .weak()
                    .italics(),
            );
            return created;
        }

        let selected = self.selected;
        let mut select = None;
        let mut remove = None;
        let mut duplicate = None;
        let mut export = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for entity in &self.entities {
                    ui.push_id(entity.id, |ui| {
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            if row(ui, entity, selected == Some(entity.id)).clicked() {
                                select = Some(entity.id);
                            }
                            ui.horizontal_wrapped(|ui| {
                                if ui.small_button("Export file").clicked() {
                                    export = Some((entity.id, ExportWhat::File));
                                }
                                if ui.small_button("Export raw body").clicked() {
                                    export = Some((entity.id, ExportWhat::Body));
                                }
                                if ui.small_button("Duplicate").clicked() {
                                    duplicate = Some(entity.id);
                                }
                                if ui.small_button("Remove").clicked() {
                                    remove = Some(entity.id);
                                }
                            });
                        });
                    });
                }
            });

        if let Some(id) = select {
            self.selected = Some(id);
        }
        if let Some((id, what)) = export {
            self.export(id, what);
        }
        if let Some(id) = duplicate {
            self.duplicate(id, log);
        }
        if let Some(id) = remove {
            self.remove(id, log);
        }
        created
    }
}

/// One workspace row: name, kind chip, origin, dirty dot, verify badge.
fn row(ui: &mut egui::Ui, entity: &LocalEntity, selected: bool) -> egui::Response {
    let title = ui.selectable_label(
        selected,
        egui::RichText::new(format!("{}  {}", &entity.name, entity.tag())).strong(),
    );
    ui.horizontal_wrapped(|ui| {
        if entity.dirty {
            ui.label(egui::RichText::new("●").small().color(crate::app::WARN))
                .on_hover_text("edited since it was loaded");
        }
        ui.label(egui::RichText::new(entity.origin.label()).small().weak());
        ui.label(
            egui::RichText::new(entity.verify.badge())
                .small()
                .color(entity.verify.color()),
        )
        .on_hover_text(entity.verify.detail());
    });
    if let Some(e) = &entity.parse_error {
        ui.label(egui::RichText::new(e).small().color(crate::app::BAD));
    }
    title
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
