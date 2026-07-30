//! The device tab: run a read-only `nord-usb` operation and show both the answer and
//! the bytes it took to get there.
//!
//! Nothing here can modify an instrument. `status`, `inventory` and `read_program` are
//! the read-only operations; the mutating ones exist in the library and are deliberately
//! not wired up.

use std::cell::RefCell;
use std::rc::Rc;

use egui::{Color32, RichText, Ui};
use nord_usb::transport::Transport;
#[cfg(not(target_arch = "wasm32"))]
use nord_usb::wire::Location;
use nord_usb::wire::{Message, Status};
use nord_usb::{ObjectClass, ReplayTransport, Session};

use crate::exec::block_on;
use crate::theme;

/// A real program-class transaction, captured off the wire from Nord Sound Manager:
/// the UI handshake, `SESSION_OPEN` for class 4, `STATUS`, then the close. Framing only
/// — it carries no instrument content.
///
/// It is replayed in **exact** mode, so it is not a canned answer: every byte this app
/// transmits is compared against what the real host sent, and a mismatch anywhere in
/// the encoder shows up here as an error rather than a plausible-looking number.
const CAPTURE: &str = "\
# host                                    device
O 0000001200000006000000010000000006a1
I 000000160000000600000001000000010000000044ec
O 000000160000000c0000000a0000000400000004a218
I 0000001a0000000c0000000a00000005000000000000000467b0
O 000000160000000c0000000a00000008000000042933
I 0000002a0000000c0000000a00000009000000000000017700000dc50000ce8b0000000000000000ac2e
O 000000120000000c0000000a000000066500
I 000000160000000c0000000a00000007000000000c4e
O 0000001200000006000000010000000226e3
I 0000001600000006000000010000000300000000006f";

/// One frame as it crossed the wire.
pub struct Wire {
    pub out: bool,
    pub bytes: Vec<u8>,
}

/// A transport decorator that records both directions. The protocol layer neither knows
/// nor cares, which is the whole point of `Transport` being one small trait.
struct Tap<T> {
    inner: T,
    log: Rc<RefCell<Vec<Wire>>>,
}

impl<T: Transport> Transport for Tap<T> {
    async fn write(&mut self, buf: &[u8]) -> nord_usb::Result<()> {
        self.log.borrow_mut().push(Wire {
            out: true,
            bytes: buf.to_vec(),
        });
        self.inner.write(buf).await
    }

    async fn read(&mut self, max: usize) -> nord_usb::Result<Vec<u8>> {
        let bytes = self.inner.read(max).await?;
        self.log.borrow_mut().push(Wire {
            out: false,
            bytes: bytes.clone(),
        });
        Ok(bytes)
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Backend {
    Capture,
    #[cfg(not(target_arch = "wasm32"))]
    Usb,
}

pub struct DevicePanel {
    backend: Backend,
    log: Rc<RefCell<Vec<Wire>>>,
    report: Vec<Status>,
    note: Option<Result<String, String>>,
    /// Which slot to fetch. Only the USB backend can read one, so the browser build
    /// has no use for it.
    #[cfg(not(target_arch = "wasm32"))]
    bank: u32,
    #[cfg(not(target_arch = "wasm32"))]
    slot: u32,
    /// A program read off the instrument, for the app to hand to the panel tab.
    pub fetched: Option<(String, Vec<u8>)>,
}

impl Default for DevicePanel {
    fn default() -> Self {
        Self {
            backend: Backend::Capture,
            log: Rc::new(RefCell::new(Vec::new())),
            report: Vec::new(),
            note: None,
            #[cfg(not(target_arch = "wasm32"))]
            bank: 1,
            #[cfg(not(target_arch = "wasm32"))]
            slot: 1,
            fetched: None,
        }
    }
}

impl DevicePanel {
    pub fn ui(&mut self, ui: &mut Ui) {
        theme::card_ui(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("transport").color(theme::DIM));
                ui.selectable_value(&mut self.backend, Backend::Capture, "recorded capture");
                #[cfg(not(target_arch = "wasm32"))]
                ui.selectable_value(&mut self.backend, Backend::Usb, "usb");
            });

            match self.backend {
                Backend::Capture => {
                    ui.label(
                        RichText::new(
                            "Replays a real NSM transaction in exact mode — every byte this app \
                             sends is checked against the capture. No hardware, and it runs \
                             unchanged in the browser.",
                        )
                        .color(theme::DIM)
                        .size(12.0),
                    );
                    ui.add_space(4.0);
                    if ui.button("run program STATUS").clicked() {
                        self.run_capture();
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                Backend::Usb => self.usb_controls(ui),
            }
        });

        if let Some(note) = &self.note {
            ui.add_space(8.0);
            let (text, color) = match note {
                Ok(msg) => (msg.as_str(), theme::AMBER),
                Err(msg) => (msg.as_str(), theme::RED_TEXT),
            };
            ui.label(RichText::new(text).color(color));
        }

        if !self.report.is_empty() {
            ui.add_space(8.0);
            theme::card_ui(ui, |ui| self.report_ui(ui));
        }

        ui.add_space(8.0);
        theme::card_ui(ui, |ui| self.log_ui(ui));
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn usb_controls(&mut self, ui: &mut Ui) {
        let attached = nord_usb::transport::usb::list().unwrap_or_default();
        if attached.is_empty() {
            ui.label(
                RichText::new("No Clavia device attached (vendor 0x0ffc).")
                    .color(theme::DIM)
                    .size(12.0),
            );
        } else {
            for info in &attached {
                ui.label(
                    RichText::new(format!(
                        "{} — {:04x}:{:04x}",
                        info.product_string().unwrap_or("Clavia device"),
                        info.vendor_id(),
                        info.product_id(),
                    ))
                    .color(theme::DIM)
                    .size(12.0),
                );
            }
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("inventory").clicked() {
                self.run_usb_inventory();
            }
            ui.separator();
            ui.label(RichText::new("read").color(theme::DIM));
            ui.add(
                egui::DragValue::new(&mut self.bank)
                    .range(1..=8)
                    .prefix("bank "),
            );
            ui.add(
                egui::DragValue::new(&mut self.slot)
                    .range(1..=50)
                    .prefix("slot "),
            );
            if ui.button("fetch program").clicked() {
                self.run_usb_read();
            }
        });
        ui.label(
            RichText::new("Read-only. Blocks the frame while the transfer runs.")
                .color(theme::DIM)
                .size(12.0),
        );
    }

    fn report_ui(&self, ui: &mut Ui) {
        ui.label(RichText::new("STATUS").color(theme::DIM).size(12.0));
        ui.add_space(4.0);
        egui::Grid::new("status_grid")
            .num_columns(4)
            .spacing([24.0, 4.0])
            .show(ui, |ui| {
                for s in &self.report {
                    ui.label(RichText::new(s.class.label()).strong());
                    match s.slots() {
                        Some(slots) => {
                            ui.label(format!("{} / {} slots", s.count, slots));
                            ui.label(format!("{} blocks each", s.blocks_per_item().unwrap_or(0)));
                        }
                        None => {
                            ui.label(format!("{} items", s.count));
                            ui.label(format!("{} / {} blocks", s.used, s.total()));
                        }
                    }
                    ui.label(
                        RichText::new(format!("{:.1}% full", s.used_percent())).color(theme::AMBER),
                    );
                    ui.end_row();
                }
            });
    }

    fn log_ui(&self, ui: &mut Ui) {
        let log = self.log.borrow();
        ui.horizontal(|ui| {
            ui.label(RichText::new("wire log").color(theme::DIM).size(12.0));
            ui.label(
                RichText::new(format!("{} frames", log.len()))
                    .color(theme::DIM)
                    .size(12.0),
            );
        });
        ui.add_space(4.0);
        if log.is_empty() {
            ui.label(RichText::new("nothing sent yet").color(theme::DIM));
            return;
        }
        egui::ScrollArea::vertical()
            .max_height(280.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for frame in log.iter() {
                    ui.horizontal(|ui| {
                        let (arrow, color) = if frame.out {
                            ("host →", theme::RED_TEXT)
                        } else {
                            ("← dev ", Color32::from_rgb(0x6c, 0xb0, 0xd8))
                        };
                        ui.label(RichText::new(arrow).monospace().color(color));
                        ui.label(RichText::new(describe(&frame.bytes)).monospace().size(12.0));
                    });
                    ui.label(
                        RichText::new(hex(&frame.bytes))
                            .monospace()
                            .size(11.0)
                            .color(theme::DIM),
                    );
                }
            });
    }

    fn run_capture(&mut self) {
        self.reset();
        let transport = match ReplayTransport::from_script(CAPTURE) {
            Ok(t) => t,
            Err(e) => {
                self.note = Some(Err(format!("bad capture script: {e}")));
                return;
            }
        };
        let mut tap = Tap {
            inner: transport,
            log: self.log.clone(),
        };
        match block_on(program_status(&mut tap)) {
            Ok(status) => {
                self.report = vec![status];
                let exhausted = tap.inner.is_exhausted();
                self.note = Some(Ok(if exhausted {
                    "every byte matched the capture, and the whole exchange was consumed".into()
                } else {
                    "matched so far, but the capture had steps left over".into()
                }));
            }
            Err(e) => self.note = Some(Err(e)),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn run_usb_inventory(&mut self) {
        self.reset();
        let transport = match nord_usb::transport::UsbTransport::open_first() {
            Ok(t) => t,
            Err(e) => {
                self.note = Some(Err(e.to_string()));
                return;
            }
        };
        let mut tap = Tap {
            inner: transport,
            log: self.log.clone(),
        };
        match block_on(nord_usb::op::inventory(&mut tap)) {
            Ok(report) => {
                self.note = Some(Ok(format!("{} classes answered", report.len())));
                self.report = report;
            }
            Err(e) => self.note = Some(Err(e.to_string())),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn run_usb_read(&mut self) {
        self.reset();
        let (bank, slot) = (self.bank, self.slot);
        let transport = match nord_usb::transport::UsbTransport::open_first() {
            Ok(t) => t,
            Err(e) => {
                self.note = Some(Err(e.to_string()));
                return;
            }
        };
        let mut tap = Tap {
            inner: transport,
            log: self.log.clone(),
        };
        match block_on(read_program(&mut tap, Location::from_user(bank, slot))) {
            Ok(file) => {
                self.note = Some(Ok(format!("read {} bytes from {bank}-{slot}", file.len())));
                self.fetched = Some((format!("{bank}-{slot}.ne5p"), file));
            }
            Err(e) => self.note = Some(Err(e)),
        }
    }

    fn reset(&mut self) {
        self.log.borrow_mut().clear();
        self.report.clear();
        self.note = None;
    }
}

/// Open a program-class session, ask for the counters, close.
///
/// The close runs whether or not the operation succeeded: an abandoned session leaves
/// the instrument mid-transaction, and the operation's own error is the informative one
/// when both fail.
async fn program_status<T: Transport>(transport: &mut T) -> Result<Status, String> {
    let mut session = Session::open(transport, ObjectClass::Program)
        .await
        .map_err(|e| e.to_string())?;
    let result = nord_usb::op::status(&mut session).await;
    let closed = session.commit().await;
    match result {
        Ok(status) => closed.map(|()| status).map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// Read one slot into the bytes of a `.ne5p` file, with the same close discipline.
#[cfg(not(target_arch = "wasm32"))]
async fn read_program<T: Transport>(transport: &mut T, at: Location) -> Result<Vec<u8>, String> {
    let mut session = Session::open(transport, ObjectClass::Program)
        .await
        .map_err(|e| e.to_string())?;
    let result = nord_usb::op::read_program(&mut session, at).await;
    let closed = session.commit().await;
    match result {
        Ok(file) => closed.map(|()| file).map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// A one-line summary of a frame, decoded rather than guessed at.
fn describe(bytes: &[u8]) -> String {
    match Message::decode(bytes) {
        Ok(m) => format!(
            "{:?} sub {} cmd {:#04x}  {} byte payload",
            m.service,
            m.subsystem,
            m.command,
            m.payload().len(),
        ),
        Err(_) => format!("{} bytes (undecodable)", bytes.len()),
    }
}

fn hex(bytes: &[u8]) -> String {
    const LIMIT: usize = 32;
    let mut s: String = bytes
        .iter()
        .take(LIMIT)
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    if bytes.len() > LIMIT {
        s.push_str(&format!(" … (+{} bytes)", bytes.len() - LIMIT));
    }
    s
}
