//! Everything the engineering build showed, one disclosure down.
//!
//! Nothing here is a control. It is the record: what the container says, what the bytes
//! did, what every field holds and how it is placed, and — for something read off the
//! instrument — what the instrument says about the slot it came from.

use eframe::egui;
use nord_format::fields::Field;
use nord_usb::{Location, ObjectClass};

use crate::app::BAD;
use crate::device::{Device, DeviceCmd};
use crate::fields::byte_diff;
use crate::strings;
use crate::workspace::LocalEntity;

/// A read the disclosure asked the instrument for.
pub struct SlotDetails {
    pub class: ObjectClass,
    pub at: Location,
}

#[derive(Default)]
pub struct Advanced {
    open: bool,
    /// The entity the cached dump belongs to.
    ///
    /// ⚠️ `{:#?}` over an undecoded body prints every byte, and a piano library is
    /// hundreds of megabytes — it is rendered once and kept, never per frame.
    dump_for: Option<u64>,
    dump: String,
}

impl Advanced {
    /// Start expanded. The disclosure is otherwise only opened by clicking it, which a
    /// headless render cannot do.
    #[cfg(test)]
    pub(super) fn start_open(&mut self) {
        self.open = true;
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        entity: &LocalEntity,
        opened: &[u8],
        fields: Option<&[Field]>,
        device: &Device,
    ) -> Option<SlotDetails> {
        let mut asked = None;
        ui.add_space(8.0);
        let header = egui::CollapsingHeader::new("Advanced")
            .id_salt("advanced")
            .default_open(self.open)
            .show(ui, |ui| {
                verify(ui, entity);
                container(ui, entity);
                diff(ui, entity, opened);
                if let Some(fields) = fields {
                    registry(ui, fields);
                }
                asked = slot(ui, entity, device);
                self.dump(ui, entity);
            });
        self.open = header.openness > 0.5;
        asked
    }

    fn dump(&mut self, ui: &mut egui::Ui, entity: &LocalEntity) {
        let Some(decoded) = &entity.entity else {
            return;
        };
        egui::CollapsingHeader::new("Raw debug")
            .id_salt("raw_debug")
            .show(ui, |ui| {
                if self.dump_for != Some(entity.id) {
                    self.dump = format!("{decoded:#?}");
                    self.dump_for = Some(entity.id);
                }
                egui::ScrollArea::both()
                    .max_height(360.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&self.dump).monospace().small());
                    });
            });
    }
}

fn verify(ui: &mut egui::Ui, entity: &LocalEntity) {
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("verify").weak());
        ui.label(
            egui::RichText::new(entity.verify.badge())
                .strong()
                .color(entity.verify.color()),
        );
        ui.label(egui::RichText::new(entity.verify.detail()).weak());
    });
    if let Some(e) = &entity.parse_error {
        ui.label(egui::RichText::new(e).color(BAD));
    }
}

fn row(ui: &mut egui::Ui, label: &str, value: impl Into<String>) {
    ui.label(egui::RichText::new(label).weak());
    ui.label(egui::RichText::new(value.into()).monospace());
    ui.end_row();
}

fn container(ui: &mut egui::Ui, entity: &LocalEntity) {
    let Some(container) = &entity.container else {
        return;
    };
    egui::CollapsingHeader::new("CBIN header")
        .id_salt("cbin")
        .show(ui, |ui| {
            egui::Grid::new("cbin_grid").num_columns(2).show(ui, |ui| {
                row(
                    ui,
                    "generation",
                    format!("{:?}", container.header.generation),
                );
                row(ui, "format", container.tag());
                row(ui, "version", container.header.version.to_string());
                row(ui, "slot", stored_slot(&container.header));
                row(ui, "body", format!("{} bytes", container.body_len));
                row(
                    ui,
                    container.checksum_label.trim_end_matches(':'),
                    match container.checksum_ok {
                        true => container.checksum.clone(),
                        false => format!("{} (does not match the bytes)", container.checksum),
                    },
                );
            });
        });
}

/// The stored slot, one-indexed as `BANK:SLOT`.
///
/// Library files carry `0xffff:0xffff` where slot files keep a bank/slot pair — a
/// library object has no slot until an instrument gives it one.
fn stored_slot(header: &nord_format::cbin::Header) -> String {
    match header.slot() {
        (0xffff, 0xffff) => "none (a library file, not a slot save)".into(),
        (bank, slot) => format!("{}:{}", bank + 1, slot + 1),
    }
}

fn diff(ui: &mut egui::Ui, entity: &LocalEntity, opened: &[u8]) {
    let rows = byte_diff(opened, &entity.bytes);
    let title = match rows.len() {
        0 => "Bytes changed since it was opened".to_string(),
        n => format!("Bytes changed since it was opened ({n})"),
    };
    egui::CollapsingHeader::new(title)
        .id_salt("bytediff")
        .show(ui, |ui| {
            if rows.is_empty() {
                ui.label(
                    egui::RichText::new(match opened.len() == entity.bytes.len() {
                        true => "nothing moved",
                        // Nothing here can pair the bytes up across a length change.
                        false => "the length changed, so there is nothing to line up",
                    })
                    .weak()
                    .small(),
                );
            }
            for row in rows {
                ui.label(
                    egui::RichText::new(format!(
                        "byte {:#06x}  {:#04x} -> {:#04x}{}",
                        row.at, row.before, row.after, row.note,
                    ))
                    .monospace()
                    .small()
                    .weak(),
                );
            }
        });
}

/// Every declared field: its path, what it holds, and where its bits sit.
fn registry(ui: &mut egui::Ui, fields: &[Field]) {
    egui::CollapsingHeader::new("Fields")
        .id_salt("registry")
        .show(ui, |ui| {
            egui::Grid::new("registry_grid")
                .num_columns(4)
                .striped(true)
                .show(ui, |ui| {
                    for field in fields {
                        ui.label(egui::RichText::new(&field.path).monospace().small());
                        // The settable spelling, which for a register is its hex.
                        ui.label(egui::RichText::new(&field.value).monospace().small());
                        ui.label(egui::RichText::new(&field.display).small().weak());
                        ui.label(
                            egui::RichText::new(field.spec.placement)
                                .monospace()
                                .small()
                                .weak(),
                        );
                        ui.end_row();
                    }
                });
        });
}

/// What the instrument says about the slot this came off.
fn slot(ui: &mut egui::Ui, entity: &LocalEntity, device: &Device) -> Option<SlotDetails> {
    let (class, at) = entity.origin.slot()?;
    let mut asked = None;
    egui::CollapsingHeader::new("On the instrument")
        .id_salt("slotdetail")
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(strings::place(class, at))
                    .monospace()
                    .small(),
            );
            let busy = device.state.in_flight.is_some();
            if ui
                .add_enabled(
                    device.state.connected() && !busy,
                    egui::Button::new("Read slot details"),
                )
                .on_disabled_hover_text("needs the instrument attached and idle")
                .clicked()
            {
                asked = Some(SlotDetails { class, at });
            }
            if device.state.detail.at != Some(at) {
                return;
            }
            match (&device.state.detail.info, device.state.detail.asked) {
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
                    ui.label(egui::RichText::new("the slot is empty").weak());
                }
                (None, false) => {}
            }
            if let Some(deps) = &device.state.detail.deps {
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
    asked
}

/// The two reads the disclosure asks for, in the order the CLI asks them.
pub fn commands(details: SlotDetails) -> [DeviceCmd; 2] {
    let SlotDetails { class, at } = details;
    [
        DeviceCmd::SlotInfo { class, at },
        DeviceCmd::Deps { class, at },
    ]
}
