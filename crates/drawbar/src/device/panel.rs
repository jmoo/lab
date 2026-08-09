//! The instrument pane.
//!
//! Read-only by default: every mutation goes through [`Confirm`], which names the thing
//! about to be lost before offering the button that loses it. The pane refuses to raise
//! a confirmation it cannot fill in — a destination whose bank has not been scanned has
//! no named occupant, so the action that would replace it stays disabled.

use eframe::egui;
use nord_usb::{Location, ObjectClass};

use super::{
    put_refusal, slots_per_bank, stem, worker, Confirm, Connection, Device, DeviceCard, DeviceCmd,
    DeviceState,
};
use crate::app::{BAD, GOOD, WARN};
use crate::log::Log;
use crate::workspace::{shown, Workspace};

/// The class tabs, in the order the pane shows them: the inventory classes, then the
/// two singletons that answer but are not slot-counted storage.
const TABS: [ObjectClass; 6] = [
    ObjectClass::Piano,
    ObjectClass::Sample,
    ObjectClass::Program,
    ObjectClass::SetList,
    ObjectClass::Live,
    ObjectClass::Settings,
];

/// What the cache last saw in a slot.
///
/// Copied out of the cache rather than borrowed from it, so naming the victim does not
/// pin `DeviceState` for as long as the pane is editing its own widget state.
#[derive(Clone)]
enum Occupant {
    Held {
        name: String,
        body_len: u32,
    },
    Empty,
    /// The bank has not been scanned, so nothing can be said about it.
    Unknown,
}

impl Occupant {
    fn name(&self) -> Option<&str> {
        match self {
            Occupant::Held { name, .. } => Some(name.as_str()),
            _ => None,
        }
    }

    fn known(&self) -> bool {
        !matches!(self, Occupant::Unknown)
    }
}

fn occupant(state: &DeviceState, class: ObjectClass, at: Location) -> Occupant {
    match state.slot(class, at) {
        Some(Some(info)) => Occupant::Held {
            name: info.name.clone(),
            body_len: info.body_len,
        },
        Some(None) => Occupant::Empty,
        None => Occupant::Unknown,
    }
}

impl Device {
    pub fn ui(&mut self, ui: &mut egui::Ui, workspace: &Workspace, log: &mut Log) {
        self.confirm_modal(ui.ctx(), log);

        let card = match &self.state.connection {
            Connection::Disconnected => return self.disconnected(ui, log),
            Connection::Connecting => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("waiting for the instrument…");
                });
                return;
            }
            Connection::Connected(card) => card.clone(),
        };

        self.header(ui, &card, log);
        ui.separator();
        self.tabs(ui);
        let class = self.pane.tab;
        self.inventory_header(ui, class);
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                self.bank_browser(ui, class, log);
                self.slot_actions(ui, class, workspace, log);
                self.detail(ui);
                self.sweep(ui, class, log);
            });
    }

    fn disconnected(&mut self, ui: &mut egui::Ui, log: &mut Log) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("●").color(egui::Color32::GRAY));
                ui.label("No instrument attached");
            });
            if ui.button("Connect").clicked() {
                // ⚠️ Reached inside the frame the click landed in, which is what keeps
                // the browser's transient user activation alive for `requestDevice()`.
                self.connect(log);
            }
            ui.separator();
            ui.label(
                egui::RichText::new(
                    "Close Nord Sound Manager first — it claims the vendor interface \
                     exclusively, and nothing else can attach alongside it.",
                )
                .small()
                .weak(),
            );
            ui.label(
                egui::RichText::new(
                    "In a browser: Chrome or Edge only. Firefox and Safari have declined \
                     WebUSB.",
                )
                .small()
                .weak(),
            );
        });
    }

    fn header(&mut self, ui: &mut egui::Ui, card: &DeviceCard, log: &mut Log) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("●").color(GOOD));
                ui.label(egui::RichText::new(&card.product).strong());
                ui.label(
                    egui::RichText::new(format!("{:04x}:{:04x}", card.vendor_id, card.product_id))
                        .monospace()
                        .weak(),
                );
            });
            ui.horizontal_wrapped(|ui| {
                if let Some(maker) = &card.manufacturer {
                    ui.label(egui::RichText::new(maker).small().weak());
                }
                if let Some(serial) = &card.serial {
                    ui.label(egui::RichText::new(serial).small().monospace().weak());
                }
            });

            let busy = self.state.in_flight.clone();
            ui.horizontal_wrapped(|ui| {
                ui.add_enabled_ui(busy.is_none(), |ui| {
                    if ui.button("Disconnect").clicked() {
                        self.disconnect(log);
                    }
                    if ui.button("Refresh").clicked() {
                        self.refresh(log);
                    }
                });
            });
            if let Some(what) = &busy {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(egui::RichText::new(what).monospace().small());
                });
                ui.label(
                    egui::RichText::new("progress is painted on the instrument's own display")
                        .small()
                        .weak(),
                );
            }
            if self.state.stale {
                ui.label(
                    egui::RichText::new(
                        "⚠ the instrument changed under us — Refresh before trusting these names",
                    )
                    .small()
                    .color(WARN),
                );
            }
        });
    }

    /// Drop every cached name and ask again. The nudge the staleness flag raises.
    fn refresh(&mut self, log: &mut Log) {
        self.state.banks.clear();
        self.state.stale = false;
        self.send(DeviceCmd::Inventory, log);
    }

    fn tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for class in TABS {
                if ui
                    .selectable_label(self.pane.tab == class, class.label())
                    .clicked()
                {
                    self.pane.tab = class;
                    self.pane.selected = None;
                }
            }
            let other = !TABS.contains(&self.pane.tab);
            if ui.selectable_label(other, "other…").clicked() {
                self.pane.tab = ObjectClass::from_raw(self.pane.other_class);
                self.pane.selected = None;
            }
            if other
                && ui
                    .add(egui::DragValue::new(&mut self.pane.other_class).range(0..=99))
                    .changed()
            {
                self.pane.tab = ObjectClass::from_raw(self.pane.other_class);
                self.pane.selected = None;
            }
        });
    }

    fn inventory_header(&mut self, ui: &mut egui::Ui, class: ObjectClass) {
        match self.state.inventory.iter().find(|s| s.class == class) {
            Some(status) => {
                ui.label(
                    egui::RichText::new(worker::describe(status))
                        .monospace()
                        .small(),
                );
            }
            // Live and Settings answer a session but report no items — they are
            // singletons, not slot-counted storage.
            None => {
                ui.label(
                    egui::RichText::new("no inventory for this class")
                        .small()
                        .weak(),
                );
            }
        }
        if let Some(why) = put_refusal(class) {
            ui.label(egui::RichText::new(why).small().color(WARN));
        }
    }

    fn bank_browser(&mut self, ui: &mut egui::Ui, class: ObjectClass, log: &mut Log) {
        let per = slots_per_bank(class);
        let busy = self.state.in_flight.is_some();
        let mut scan = false;
        ui.horizontal_wrapped(|ui| {
            ui.label("bank");
            ui.add(egui::DragValue::new(&mut self.pane.bank).range(1..=99));
            ui.add_enabled_ui(!busy, |ui| {
                scan = ui
                    .button("Scan bank")
                    .on_hover_text("one INFO per slot, in a single session")
                    .clicked();
            });
        });
        if scan {
            self.send(
                DeviceCmd::ScanBank {
                    class,
                    bank: self.pane.bank,
                    slots: per,
                },
                log,
            );
        }

        let bank = self.pane.bank;
        let Some(slots) = self.state.bank(class, bank) else {
            ui.label(
                egui::RichText::new("this bank has not been scanned")
                    .small()
                    .weak(),
            );
            return;
        };

        let selected = self.pane.selected;
        let mut click = None;
        egui::Grid::new("slots")
            .num_columns(3)
            .striped(true)
            .show(ui, |ui| {
                for (i, slot) in slots.iter().enumerate() {
                    let at = Location {
                        bank: bank - 1,
                        slot: i as u32,
                    };
                    let label = egui::RichText::new(shown(at)).monospace();
                    if ui.selectable_label(selected == Some(at), label).clicked() {
                        click = Some(at);
                    }
                    match slot {
                        Some(info) => {
                            ui.label(&info.name);
                            ui.label(egui::RichText::new(&info.format).monospace().small().weak());
                        }
                        // A vacant slot is a row, not an absence: it is a valid target
                        // for a put or a move.
                        None => {
                            ui.label(egui::RichText::new("(empty)").weak().italics());
                            ui.label("");
                        }
                    }
                    ui.end_row();
                }
            });
        if let Some(at) = click {
            self.pane.selected = Some(at);
            self.pane.rename_to = match occupant(&self.state, class, at) {
                Occupant::Held { name, .. } => name,
                _ => String::new(),
            };
        }
    }

    fn slot_actions(
        &mut self,
        ui: &mut egui::Ui,
        class: ObjectClass,
        workspace: &Workspace,
        log: &mut Log,
    ) {
        let Some(at) = self.pane.selected else {
            return;
        };
        ui.separator();
        let held = occupant(&self.state, class, at);
        let name = held.name().unwrap_or("(empty)").to_string();
        // A read is offered unless the slot is known to be vacant; a mutation needs the
        // occupant *named*, which an unscanned bank cannot do.
        let readable = !matches!(held, Occupant::Empty);
        let named = held.name().is_some();
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(shown(at)).monospace().strong());
            ui.label(&name);
        });

        let busy = self.state.in_flight.is_some();
        let mut cmd = None;
        let mut confirm = None;

        ui.add_enabled_ui(!busy, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(readable, egui::Button::new("Get")).clicked() {
                    cmd = Some(DeviceCmd::Get {
                        class,
                        at,
                        body: false,
                        label: None,
                    });
                }
                if ui
                    .add_enabled(readable, egui::Button::new("Get raw body"))
                    .on_hover_text("the wire body verbatim, with no CBIN header wrapped round it")
                    .clicked()
                {
                    cmd = Some(DeviceCmd::Get {
                        class,
                        at,
                        body: true,
                        label: None,
                    });
                }
                if ui.button("Info").clicked() {
                    cmd = Some(DeviceCmd::SlotInfo { class, at });
                }
                if ui
                    .add_enabled(readable, egui::Button::new("Deps"))
                    .clicked()
                {
                    cmd = Some(DeviceCmd::Deps { class, at });
                }
                if ui
                    .add_enabled(readable, egui::Button::new("Select"))
                    .on_hover_text("load it live on the instrument; nothing stored changes")
                    .clicked()
                {
                    cmd = Some(DeviceCmd::Select { class, at });
                }
            });

            ui.horizontal_wrapped(|ui| {
                ui.label("rename");
                ui.add(
                    egui::TextEdit::singleline(&mut self.pane.rename_to)
                        .desired_width(140.0)
                        .hint_text("new name"),
                );
                let ready = named && !self.pane.rename_to.trim().is_empty();
                if ui.add_enabled(ready, egui::Button::new("Rename")).clicked() {
                    let to = self.pane.rename_to.trim().to_string();
                    confirm = Some(Confirm {
                        title: format!("Rename {}?", shown(at)),
                        lines: vec![format!("{name:?} becomes {to:?} on the instrument.")],
                        cmd: DeviceCmd::Rename {
                            class,
                            at,
                            name: to,
                        },
                    });
                }
            });

            ui.horizontal_wrapped(|ui| {
                ui.label("to");
                ui.add(egui::DragValue::new(&mut self.pane.dest_bank).range(1..=99));
                ui.label(":");
                ui.add(egui::DragValue::new(&mut self.pane.dest_slot).range(1..=99));
                let to = Location::from_user(self.pane.dest_bank, self.pane.dest_slot);
                let dest = occupant(&self.state, class, to);
                // The destination has to be named before it can be replaced, and only a
                // scan can name it. Stricter than the CLI, which proceeds after a peek
                // that failed — here an unnamed victim is simply not offered.
                let ready = named && dest.known();
                let why = match named {
                    false => "the source slot has no named occupant".to_string(),
                    true => format!(
                        "scan bank {} first — the destination's occupant has to be named",
                        self.pane.dest_bank
                    ),
                };

                if ui
                    .add_enabled(ready, egui::Button::new("Move"))
                    .on_disabled_hover_text(&why)
                    .clicked()
                {
                    confirm = Some(Confirm {
                        title: format!("Move {name:?} from {} to {}?", shown(at), shown(to)),
                        // ⚠️ Saying "overwriting" for a swap is worse than saying
                        // nothing: it invites deleting the destination first to protect
                        // it, which destroys the very thing the swap preserves.
                        lines: vec![match dest.name() {
                            Some(there) => format!(
                                "{} holds {there:?}. A move is a SWAP — it ends up in {}, \
                                 byte-identical. Confirmed on hardware.",
                                shown(to),
                                shown(at)
                            ),
                            None => format!("{} is empty.", shown(to)),
                        }],
                        cmd: DeviceCmd::Move {
                            class,
                            from: at,
                            to,
                        },
                    });
                }

                if ui
                    .add_enabled(ready, egui::Button::new("Duplicate"))
                    .on_disabled_hover_text(&why)
                    .clicked()
                {
                    confirm = Some(Confirm {
                        title: format!("Duplicate {name:?} from {} to {}?", shown(at), shown(to)),
                        lines: vec![match dest.name() {
                            Some(there) => format!(
                                "OVERWRITING {there:?} in {} — a duplicate replaces the \
                                 destination, and it is lost.",
                                shown(to)
                            ),
                            None => format!("{} is empty.", shown(to)),
                        }],
                        cmd: DeviceCmd::Duplicate {
                            class,
                            from: at,
                            to,
                        },
                    });
                }
            });

            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(named, egui::Button::new("Delete")).clicked() {
                    confirm = Some(Confirm {
                        title: format!("Delete {name:?} from {}?", shown(at)),
                        lines: vec!["It is removed from the instrument. There is no undo.".into()],
                        cmd: DeviceCmd::Delete { class, at },
                    });
                }
                if let Some(put) = put_button(ui, class, at, &held, workspace) {
                    confirm = Some(put);
                }
            });
        });

        if let Some(cmd) = cmd {
            self.send(cmd, log);
        }
        if confirm.is_some() {
            self.pane.confirm = confirm;
        }
    }

    fn detail(&mut self, ui: &mut egui::Ui) {
        let Some(at) = self.state.detail.at else {
            return;
        };
        egui::CollapsingHeader::new(format!("slot detail {}", shown(at)))
            .default_open(true)
            .show(ui, |ui| {
                match (&self.state.detail.info, self.state.detail.asked) {
                    (Some(info), _) => {
                        egui::Grid::new("slot_detail")
                            .num_columns(2)
                            .show(ui, |ui| {
                                row(ui, "name", format!("{:?}", info.name));
                                row(ui, "format", info.format.clone());
                                row(ui, "version", info.version.to_string());
                                row(ui, "body", format!("{} bytes", info.body_len));
                                row(
                                    ui,
                                    "crc32",
                                    match info.crc32 {
                                        Some(crc) => format!("{crc:#010x}"),
                                        // Library content reports 0xffffffff: no checksum is
                                        // kept for objects this large.
                                        None => "none (not checksummed for this class)".into(),
                                    },
                                );
                            });
                    }
                    (None, true) => {
                        ui.label(egui::RichText::new(format!("{} is empty", shown(at))).weak());
                    }
                    (None, false) => {}
                }
                if let Some(deps) = &self.state.detail.deps {
                    ui.separator();
                    if deps.is_empty() {
                        ui.label(egui::RichText::new("no dependencies").weak());
                    }
                    egui::Grid::new("deps").num_columns(3).show(ui, |ui| {
                        for dep in deps {
                            ui.label(egui::RichText::new(dep.class.label()).small().weak());
                            ui.label(egui::RichText::new(format!("{:08x}", dep.id)).monospace());
                            // The names come from the device; a file stores ids only.
                            ui.label(dep.name.trim());
                            ui.end_row();
                        }
                    });
                }
            });
    }

    fn sweep(&mut self, ui: &mut egui::Ui, class: ObjectClass, log: &mut Log) {
        egui::CollapsingHeader::new("sweep").show(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "Change one thing on the instrument, say what it was, then capture. \
                     The read happens after the change, so each capture is filed under \
                     the state it is in.",
                )
                .small()
                .weak(),
            );
            let Some(at) = self.pane.selected else {
                ui.label(egui::RichText::new("select a slot first").small().weak());
                return;
            };
            let mut capture = None;
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.pane.sweep_label)
                        .desired_width(160.0)
                        .hint_text("what changed"),
                );
                let label = stem(&self.pane.sweep_label);
                let ready = self.state.in_flight.is_none() && !label.is_empty();
                if ui
                    .add_enabled(ready, egui::Button::new("Capture"))
                    .clicked()
                {
                    capture = Some(label);
                }
            });
            if let Some(label) = capture {
                self.send(
                    DeviceCmd::Get {
                        class,
                        at,
                        body: false,
                        label: Some(label),
                    },
                    log,
                );
            }
        });
    }

    /// The one gate to a destructive session: nothing reaches the worker's
    /// `allow_destructive_writes` except the command this modal releases.
    fn confirm_modal(&mut self, ctx: &egui::Context, log: &mut Log) {
        let Some(confirm) = &self.pane.confirm else {
            return;
        };
        let mut decision = None;
        egui::Modal::new(egui::Id::new("device_confirm")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.heading(&confirm.title);
            ui.add_space(4.0);
            for line in &confirm.lines {
                ui.label(line);
            }
            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    decision = Some(false);
                }
                if ui
                    .add(egui::Button::new(egui::RichText::new("Proceed").strong()).fill(BAD))
                    .clicked()
                {
                    decision = Some(true);
                }
            });
        });
        match decision {
            Some(true) => {
                if let Some(confirm) = self.pane.confirm.take() {
                    self.send(confirm.cmd, log);
                }
            }
            Some(false) => {
                self.pane.confirm = None;
                log.info("cancelled");
            }
            None => {}
        }
    }
}

/// The put button, and the confirmation that spells out the delete-first window.
fn put_button(
    ui: &mut egui::Ui,
    class: ObjectClass,
    at: Location,
    held: &Occupant,
    workspace: &Workspace,
) -> Option<Confirm> {
    let Some(entity) = workspace.selected() else {
        ui.add_enabled(false, egui::Button::new("Put here"))
            .on_disabled_hover_text("select an entity in the workspace first");
        return None;
    };
    let label = format!("Put {:?} here", entity.name);
    if let Some(why) = put_refusal(class) {
        ui.add_enabled(false, egui::Button::new(label))
            .on_disabled_hover_text(why);
        return None;
    }
    // Refused before the transport is touched: bytes that are not what they claim to be
    // must not reach a delete-then-write.
    if let Err(e) = nord_usb::envelope::unwrap(&entity.bytes) {
        ui.add_enabled(false, egui::Button::new(label))
            .on_disabled_hover_text(format!("{}: {e}", entity.name));
        return None;
    }
    if !ui.button(label).clicked() {
        return None;
    }

    let line = match held {
        // The operator is consenting to the slot being empty for a moment, not just to a
        // write, so the delete has to be part of the question.
        Occupant::Held { body_len, .. } => format!(
            "note: the instrument will not overwrite in place, so {} is deleted first. \
             Its {body_len} bytes are read back beforehand and put back if the write fails.",
            shown(at),
        ),
        Occupant::Empty => format!("{} is empty.", shown(at)),
        Occupant::Unknown => format!(
            "{} has not been scanned; it is read first either way, and whatever it holds \
             is backed up before anything is deleted.",
            shown(at)
        ),
    };
    Some(Confirm {
        title: match held.name() {
            Some(there) => format!("Overwrite {} {there:?} with {:?}?", shown(at), entity.name),
            None => format!("Write {:?} into {}?", entity.name, shown(at)),
        },
        lines: vec![line],
        cmd: DeviceCmd::Put {
            class,
            at,
            name: entity.name.clone(),
            bytes: entity.bytes.clone(),
        },
    })
}

fn row(ui: &mut egui::Ui, label: &str, value: String) {
    ui.label(egui::RichText::new(label).weak());
    ui.label(egui::RichText::new(value).monospace());
    ui.end_row();
}
