//! The document: one view of one asset, which looking at and changing are the same act.
//!
//! An edit lands on the tab's working copy the moment it is made — set the field,
//! re-encode, re-check the bytes — and the tab goes dirty. Nothing on the instrument
//! moves until the header's Send does it. Revert goes back to the bytes the tab opened
//! with, which is the only undo there is.

use eframe::egui;
use nord_format::fields::Field;
use nord_usb::{Location, ObjectClass};

use crate::app::{dot, BAD, WARN};
use crate::device::Device;
use crate::fields;
use crate::log::Log;
use crate::strings;
use crate::workspace::{ExportWhat, LocalEntity, Workspace};

mod advanced;
mod controls;
mod panel;
mod sample;

use advanced::Advanced;
use controls::{Ctx, Sets};

/// A put the header asked for. The browser owns the question it may need to raise.
pub struct SendBack {
    pub id: u64,
    pub class: ObjectClass,
    pub at: Location,
}

/// What the header row asked for this frame.
#[derive(Default)]
struct Header {
    revert: bool,
    save: bool,
    send: Option<SendBack>,
}

pub struct Document {
    target: Option<u64>,
    /// ⚠️ Per-field legal values, read once per document: asking a field for them walks
    /// every bit pattern it can hold.
    ctx: Ctx,
    /// The name box for a sample instrument, so a half-typed name survives a frame.
    name: String,
    /// The last refusal, and what caused it.
    error: Option<String>,
    advanced: Advanced,
}

impl Default for Document {
    fn default() -> Document {
        Document {
            target: None,
            ctx: Ctx::read(&[]),
            name: String::new(),
            error: None,
            advanced: Advanced::default(),
        }
    }
}

impl Document {
    /// Draw the open tab's document. `opened` is what it looked like when the tab
    /// opened — Revert's target, and what the byte diff is measured against.
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        id: u64,
        opened: &[u8],
        workspace: &mut Workspace,
        device: &mut Device,
        log: &mut Log,
    ) -> Option<SendBack> {
        let entity = workspace.get(id)?;
        let decoded = entity.entity.as_ref();
        let registry = decoded.map(fields::fields_of).unwrap_or_default();

        if self.target != Some(id) {
            self.target = Some(id);
            self.error = None;
            self.ctx = Ctx::read(registry.as_deref().unwrap_or(&[]));
            self.name = decoded
                .and_then(sample::snapshot)
                .and_then(Result::ok)
                .map_or(String::new(), |snapshot| snapshot.name);
        }

        let mut sets: Sets = Vec::new();
        let mut act = Header::default();

        self.header(ui, entity, opened, &mut act);
        if let Some(why) = &self.error {
            ui.label(egui::RichText::new(why).color(BAD));
        }
        ui.separator();

        let mut details = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                self.body(ui, entity, registry.as_deref(), &mut sets);
                details = self
                    .advanced
                    .ui(ui, entity, opened, registry.as_deref(), device);
            });

        if let Some(details) = details {
            for cmd in advanced::commands(details) {
                device.send(cmd, log);
            }
        }
        if act.save {
            workspace.export(id, ExportWhat::File);
        }
        if act.revert {
            workspace.restore_bytes(id, opened.to_vec(), log);
            self.error = None;
            // Reread on the next frame: the name box is holding an edit that is gone.
            self.target = None;
            return act.send;
        }
        if !sets.is_empty() {
            self.apply(id, sets, workspace, log);
        }
        act.send
    }

    /// Where it came from, what has changed, and the three things to do about it.
    fn header(&mut self, ui: &mut egui::Ui, entity: &LocalEntity, opened: &[u8], act: &mut Header) {
        ui.horizontal_wrapped(|ui| {
            ui.heading(&entity.name);
            if entity.dirty {
                dot(ui, WARN);
                ui.label(egui::RichText::new("changed").small().color(WARN));
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(entity.origin.label()).small().weak());
        });
        ui.horizontal_wrapped(|ui| {
            act.revert = ui
                .add_enabled(entity.bytes != opened, egui::Button::new("Revert"))
                .on_hover_text("back to how it was when this tab opened")
                .on_disabled_hover_text("nothing has changed")
                .clicked();
            act.save = ui.button("Save to disk…").clicked();
            if let Some((class, at)) = entity.origin.slot() {
                let refusal = crate::device::put_refusal(class);
                let sendable = refusal.is_none() && !crate::device::read_only(class);
                let label = format!("Send to {}", strings::place(class, at));
                let button = ui.add_enabled(sendable, egui::Button::new(label));
                let button = match refusal {
                    Some(why) => button.on_disabled_hover_text(why),
                    None => button.on_disabled_hover_text(format!(
                        "{} are installed on the instrument, not sent to it",
                        strings::folder(class)
                    )),
                };
                if button.clicked() {
                    act.send = Some(SendBack {
                        id: entity.id,
                        class,
                        at,
                    });
                }
            }
        });
    }

    /// Whichever shape this asset is.
    fn body(
        &mut self,
        ui: &mut egui::Ui,
        entity: &LocalEntity,
        registry: Option<&[Field]>,
        sets: &mut Sets,
    ) {
        let Some(decoded) = &entity.entity else {
            ui.label(
                egui::RichText::new(
                    "This file did not decode, so there is nothing to show but its bytes.",
                )
                .weak(),
            );
            return;
        };
        if sample::is_sample(decoded) {
            return self.sample_body(ui, decoded, sets);
        }
        if let Some(fields) = registry {
            if fields::is_electro5_panel(decoded) {
                return panel::program(ui, &self.ctx, fields, sets);
            }
            if fields::is_electro5_settings(decoded) {
                return panel::settings(ui, &self.ctx, fields, sets);
            }
            return panel::plain(ui, &self.ctx, fields, sets);
        }
        set_list(ui, decoded);
    }

    fn sample_body(&mut self, ui: &mut egui::Ui, decoded: &nord_format::Entity, sets: &mut Sets) {
        match sample::snapshot(decoded) {
            Some(Ok(snapshot)) => {
                let editable = sample::is_editable(decoded);
                sample::ui(ui, &snapshot, &mut self.name, editable, sets);
            }
            Some(Err(why)) => {
                ui.label(egui::RichText::new(why).color(BAD));
            }
            None => sample::ui(
                ui,
                &sample::Snapshot {
                    name: String::new(),
                    zones: Vec::new(),
                },
                &mut self.name,
                false,
                sets,
            ),
        }
    }

    /// Apply this frame's changes to the working copy.
    ///
    /// All of them or none: a control that owns two fields must not leave one half
    /// written when the library refuses the other.
    fn apply(&mut self, id: u64, sets: Sets, workspace: &mut Workspace, log: &mut Log) {
        let Some(entity) = workspace.get(id) else {
            return;
        };
        let bytes = entity.bytes.clone();
        let sample = entity.entity.as_ref().is_some_and(sample::is_sample);
        let result = match sample {
            true => sample::apply(&bytes, &sets),
            false => fields::apply(&bytes, &sets).map(|(_, out)| out),
        };
        match result {
            Ok(out) if out == bytes => self.error = None,
            Ok(out) => {
                self.error = None;
                workspace.replace_bytes(id, out, log);
            }
            Err(why) => {
                log.error(why.clone());
                self.error = Some(why);
            }
        }
    }
}

/// A set list: the four programs it points at. The four slots are what a set list is.
fn set_list(ui: &mut egui::Ui, entity: &nord_format::Entity) {
    let nord_format::Entity::Song(nord_format::Song::Electro5(song)) = entity else {
        ui.label(egui::RichText::new("Nothing about this format is editable here yet.").weak());
        return;
    };
    ui.label(
        egui::RichText::new("The four programs this song plays.")
            .small()
            .weak(),
    );
    for (i, at) in song.body.programs().iter().enumerate() {
        let (bank, slot) = at.inner();
        ui.horizontal(|ui| {
            ui.add_sized(
                [90.0, ui.spacing().interact_size.y],
                egui::Label::new(format!("Slot {}", i + 1)).halign(egui::Align::LEFT),
            );
            ui.label(format!("Programs {}:{}", bank + 1, slot + 1));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{Fresh, Origin};

    /// Paint one document into a headless context.
    ///
    /// Nothing checks pixels; what this catches is the failure a document can actually
    /// have — a section that indexes past its fields, or a control asked for a value the
    /// field cannot hold.
    fn render(sets: &[(&str, &str)], kind: Fresh) {
        render_with(sets, kind, false);
    }

    fn render_with(sets: &[(&str, &str)], kind: Fresh, advanced: bool) {
        let ctx = egui::Context::default();
        let mut workspace = Workspace::new(ctx.clone());
        let mut device = Device::new(ctx.clone());
        let mut log = Log::default();
        let mut document = Document::default();
        if advanced {
            document.advanced.start_open();
        }

        let id = workspace.create(kind, &mut log).expect("a fresh default");
        if !sets.is_empty() {
            let bytes = workspace.get(id).unwrap().bytes.clone();
            let sets: Vec<(String, String)> = sets
                .iter()
                .map(|(p, v)| ((*p).to_string(), (*v).to_string()))
                .collect();
            let (_, edited) = fields::apply(&bytes, &sets).expect("the sets are legal");
            workspace.replace_bytes(id, edited, &mut log);
        }
        let opened = workspace.get(id).unwrap().bytes.clone();

        // Twice: the second pass runs with the caches and the widget state the first
        // one left behind, which is where a stale index would show up.
        for _ in 0..2 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    document.ui(ui, id, &opened, &mut workspace, &mut device, &mut log);
                });
            });
        }
    }

    #[test]
    fn a_program_document_paints_for_every_organ_the_panel_offers() {
        for organ in ["B3", "B3Bass", "Vox", "Farfisa", "Pipe", "unknown (6)"] {
            render(&[("center_panel.organ_type", organ)], Fresh::Program);
        }
    }

    /// A part pointed at each instrument in turn: every section opens at least once.
    #[test]
    fn a_program_document_paints_with_each_part_in_use() {
        for instrument in ["Organ", "Piano", "Sample"] {
            render(&[("center_panel.upper_part", instrument)], Fresh::Program);
        }
    }

    /// The disclosure carries the byte diff, the registry table and the raw dump, none
    /// of which the closed pass reaches.
    #[test]
    fn the_advanced_disclosure_paints() {
        render_with(&[("center_panel.gain", "96")], Fresh::Program, true);
        render_with(&[], Fresh::Settings, true);
    }

    #[test]
    fn the_other_fresh_defaults_paint() {
        render(&[], Fresh::Live);
        render(&[], Fresh::Settings);
    }

    /// Bytes that do not decode still have a document — it says so and shows the record.
    #[test]
    fn a_file_that_did_not_decode_still_paints() {
        let ctx = egui::Context::default();
        let mut workspace = Workspace::new(ctx.clone());
        let mut device = Device::new(ctx.clone());
        let mut log = Log::default();
        let mut document = Document::default();
        let id = workspace.ingest(
            "junk.bin".into(),
            Origin::File("junk.bin".into()),
            b"not a nord file".to_vec(),
            &mut log,
        );
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                document.ui(ui, id, &[], &mut workspace, &mut device, &mut log);
            });
        });
    }
}
