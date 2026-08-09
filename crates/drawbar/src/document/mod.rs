//! The document: one view of one asset, which looking at and changing are the same act.
//!
//! An edit lands on the tab's working copy the moment it is made — set the field,
//! re-encode, re-check the bytes — and the tab goes dirty. Nothing on the instrument
//! moves until the header's Send does it. Revert goes back to the bytes the tab opened
//! with, which is the only undo there is.

use eframe::egui;
use nord_format::fields::Field;
use nord_usb::{Location, ObjectClass};

use crate::app::dot;
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

/// The body's own scroll id — see [`crate::tabs::SCROLL`].
pub const SCROLL: &str = "document_body";

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

/// Which face of a document is showing.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum View {
    /// The panel, in the instrument's own words. Happy-path edits.
    #[default]
    Basic,
    /// The whole body as a table, plus the record. Nothing hidden.
    Advanced,
}

pub struct Document {
    target: Option<u64>,
    /// Which face each document was left on.
    views: std::collections::HashMap<u64, View>,
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
            views: std::collections::HashMap::new(),
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

        // Something with no friendly view has only the record to show.
        let basic_exists = entity
            .entity
            .as_ref()
            .is_some_and(|e| fields::has_registry(e) || sample::is_sample(e));
        let view = match basic_exists {
            true => self.views.get(&id).copied().unwrap_or_default(),
            false => View::Advanced,
        };

        let mut sets: Sets = Vec::new();
        let mut act = Header::default();

        self.header(ui, entity, opened, &mut act);
        if basic_exists {
            let mut want = view;
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .selectable_label(view == View::Advanced, "Advanced")
                        .clicked()
                    {
                        want = View::Advanced;
                    }
                    if ui.selectable_label(view == View::Basic, "Basic").clicked() {
                        want = View::Basic;
                    }
                });
            });
            self.views.insert(id, want);
        }
        if let Some(why) = &self.error {
            ui.label(egui::RichText::new(why).color(crate::app::bad(ui.visuals())));
        }
        ui.separator();

        let mut details = None;
        let mut typed = false;
        let mut piano = piano_lookup(entity, registry.as_deref(), device);
        egui::ScrollArea::vertical()
            .id_salt(SCROLL)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                match view {
                    View::Basic => {
                        self.body(ui, entity, registry.as_deref(), &mut piano, &mut sets)
                    }
                    View::Advanced => {
                        if let Some(fields) = registry.as_deref() {
                            self.advanced.table(ui, fields, &mut sets);
                            typed = !sets.is_empty();
                            ui.add_space(8.0);
                        }
                    }
                }
                details = self
                    .advanced
                    .ui(ui, entity, opened, registry.as_deref(), device);
            });

        if let Some(details) = details {
            for cmd in advanced::commands(details) {
                device.send(cmd, log);
            }
        }
        if piano.asked {
            if let Some((class, at)) = workspace.get(id).and_then(|e| e.origin.slot()) {
                device.send(crate::device::DeviceCmd::Deps { class, at }, log);
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
            let outcome = self.apply(id, sets, workspace, log);
            if typed {
                // The table keeps a refused cell open with what was typed in it.
                self.advanced.settled(outcome);
            }
        }
        act.send
    }

    /// Mark the open document as owed to the instrument, or say why it is not.
    pub fn stage(&mut self, id: u64, workspace: &mut Workspace, log: &mut Log) {
        let Some(entity) = workspace.get(id) else {
            return;
        };
        let name = entity.name.clone();
        match entity.origin.slot() {
            Some((class, at)) if crate::device::sendable(class) => {
                workspace.mark_pending(id, true);
                log.say(format!(
                    "“{name}” will be sent to {}.",
                    strings::place(class, at)
                ));
            }
            // ⚠️ Never a file export. Cmd+S means "keep what I did", and for something
            // that lives here that has already happened.
            _ => log.say(format!(
                "“{name}” is on this computer, and changes to it are kept as you make them."
            )),
        }
    }

    /// Where it came from, what has changed, and the three things to do about it.
    fn header(&mut self, ui: &mut egui::Ui, entity: &LocalEntity, opened: &[u8], act: &mut Header) {
        ui.horizontal_wrapped(|ui| {
            ui.heading(&entity.name);
            if entity.dirty {
                let warn = crate::app::warn(ui.visuals());
                dot(ui, warn);
                ui.label(egui::RichText::new("changed").small().color(warn));
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
            act.save = ui.button("Export…").clicked();
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
        piano: &mut panel::PianoLookup,
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
                return panel::program(ui, &self.ctx, fields, piano, sets);
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
                ui.label(egui::RichText::new(why).color(crate::app::bad(ui.visuals())));
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
    fn apply(
        &mut self,
        id: u64,
        sets: Sets,
        workspace: &mut Workspace,
        log: &mut Log,
    ) -> Result<(), String> {
        let Some(entity) = workspace.get(id) else {
            return Ok(());
        };
        let bytes = entity.bytes.clone();
        let sample = entity.entity.as_ref().is_some_and(sample::is_sample);
        let result = match sample {
            true => sample::apply(&bytes, &sets),
            false => fields::apply(&bytes, &sets).map(|(_, out)| out),
        };
        match result {
            Ok(out) if out == bytes => {
                self.error = None;
                Ok(())
            }
            Ok(out) => {
                self.error = None;
                workspace.replace_bytes(id, out, log);
                // An edit to something read off the instrument is owed back to it. It
                // goes nowhere until the operator sends it.
                if workspace
                    .get(id)
                    .and_then(|e| e.origin.slot())
                    .is_some_and(|(class, _)| crate::device::sendable(class))
                {
                    workspace.mark_pending(id, true);
                }
                Ok(())
            }
            Err(why) => {
                log.error(why.clone());
                self.error = Some(why.clone());
                Err(why)
            }
        }
    }
}

/// What is known about the piano a program plays.
///
/// ⚠️ The name can only come from the instrument. A `.ne5p` stores the piano's **id** and
/// no name at all, and the category/model pair beside it is the panel's dial position
/// rather than an identity — so nothing here resolves a name out of the file, and a
/// document opened with no instrument attached shows the id and says so.
fn piano_lookup(
    entity: &LocalEntity,
    registry: Option<&[Field]>,
    device: &Device,
) -> panel::PianoLookup {
    let id = registry
        .and_then(|fields| fields.iter().find(|field| field.path == "piano_panel.id"))
        .and_then(|field| library_id(&field.value))
        // Zero is "this program references no piano", not an id to go looking for.
        .filter(|id| *id != 0);
    let slot = entity.origin.slot();
    panel::PianoLookup {
        id,
        name: id
            .and_then(|id| device.state.dependency_name(slot, ObjectClass::Piano, id))
            .map(str::to_string),
        can_ask: slot.is_some() && device.state.connected(),
        asked: false,
    }
}

/// A library id as the registry spells it — decimal from the field list, hex where a
/// person typed it.
fn library_id(value: &str) -> Option<u32> {
    let text = value.trim();
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
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
        render_view(sets, kind, advanced, View::Basic);
    }

    fn render_view(sets: &[(&str, &str)], kind: Fresh, advanced: bool, view: View) {
        let ctx = egui::Context::default();
        let mut workspace = Workspace::new(ctx.clone());
        let mut device = Device::new(ctx.clone());
        let mut log = Log::default();
        let mut document = Document::default();
        if advanced {
            document.advanced.start_open();
        }

        let id = workspace.create(kind, &mut log).expect("a fresh default");
        document.views.insert(id, view);
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

    /// The engineer's table paints, filters and holds an edit — for a body with ninety
    /// fields and for one with forty.
    #[test]
    fn the_advanced_table_paints() {
        render_view(&[], Fresh::Program, true, View::Advanced);
        render_view(&[], Fresh::Settings, true, View::Advanced);
    }

    /// A cell the library refuses stays open with what was typed in it, because that is
    /// the only copy of what the operator meant.
    #[test]
    fn a_refused_cell_keeps_its_error() {
        let ctx = egui::Context::default();
        let mut workspace = Workspace::new(ctx.clone());
        let mut log = Log::default();
        let mut document = Document::default();
        let id = workspace.create(Fresh::Program, &mut log).unwrap();

        // What the table does with the library's answer, which is the part worth
        // pinning: the same call the frame makes.
        let refused = document.apply(
            id,
            vec![("center_panel.gain".into(), "200".into())],
            &mut workspace,
            &mut log,
        );
        assert!(refused.is_err());
        document.advanced.settled(refused);
        assert!(document.error.is_some());

        let taken = document.apply(
            id,
            vec![("center_panel.gain".into(), "96".into())],
            &mut workspace,
            &mut log,
        );
        assert!(taken.is_ok());
        document.advanced.settled(taken);
        assert!(document.error.is_none());
    }

    /// An edit to something read off the instrument is owed back to it; an edit to
    /// something that only lives here is not.
    #[test]
    fn editing_a_device_document_marks_it_pending() {
        use crate::workspace::Origin;
        use nord_usb::{Location, ObjectClass};

        let ctx = egui::Context::default();
        let mut workspace = Workspace::new(ctx.clone());
        let mut log = Log::default();
        let mut document = Document::default();

        let local = workspace.create(Fresh::Program, &mut log).unwrap();
        let bytes = workspace.get(local).unwrap().bytes.clone();
        let from_device = workspace.ingest(
            "Africa-Split.ne5p".into(),
            Origin::Device {
                class: ObjectClass::Program,
                at: Location { bank: 6, slot: 3 },
            },
            bytes,
            &mut log,
        );

        let set = vec![("center_panel.gain".to_string(), "96".to_string())];
        document
            .apply(local, set.clone(), &mut workspace, &mut log)
            .unwrap();
        document
            .apply(from_device, set, &mut workspace, &mut log)
            .unwrap();

        assert!(!workspace.get(local).unwrap().pending, "stays here");
        assert!(workspace.get(from_device).unwrap().pending, "owed back");
        let owed: Vec<u64> = workspace.pending().iter().map(|e| e.id).collect();
        assert_eq!(owed, vec![from_device]);

        // Cmd+S on the local one says so rather than exporting anything.
        document.stage(local, &mut workspace, &mut log);
        assert!(
            log.status().1.contains("kept as you make them"),
            "{}",
            log.status().1
        );
        assert!(!workspace.get(local).unwrap().pending);
    }

    /// ⚠️ The strip and the body are two scroll regions in one `Ui`. While they shared
    /// egui's unsalted id they shared one state, and a wheel over the document moved the
    /// tab strip while the body stayed where it was.
    #[test]
    fn the_tab_strip_and_the_document_body_scroll_on_their_own() {
        let ctx = egui::Context::default();
        let mut workspace = Workspace::new(ctx.clone());
        let mut device = Device::new(ctx.clone());
        let mut log = Log::default();
        let mut document = Document::default();
        let mut tabs = crate::tabs::Tabs::default();

        let id = workspace.create(Fresh::Program, &mut log).unwrap();
        tabs.open(id, &workspace);
        let opened = workspace.get(id).unwrap().bytes.clone();

        let mut ids = None;
        for frame in 0..4 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 720.0),
                )),
                // The first frame lays the document out; the rest wheel over its middle.
                events: match frame {
                    0 => Vec::new(),
                    _ => vec![
                        egui::Event::PointerMoved(egui::pos2(900.0, 400.0)),
                        egui::Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: egui::vec2(0.0, -200.0),
                            modifiers: egui::Modifiers::default(),
                        },
                    ],
                },
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ids = Some((
                        ui.make_persistent_id(egui::Id::new(crate::tabs::SCROLL)),
                        ui.make_persistent_id(egui::Id::new(SCROLL)),
                    ));
                    tabs.ui(ui, &workspace);
                    ui.separator();
                    document.ui(ui, id, &opened, &mut workspace, &mut device, &mut log);
                });
            });
        }

        let (strip, body) = ids.expect("the panel drew");
        assert_ne!(strip, body, "one state each");
        let offset = |id| egui::scroll_area::State::load(&ctx, id).map(|state| state.offset.y);
        assert_eq!(offset(strip), Some(0.0), "the strip has nothing to scroll");
        assert!(
            offset(body).is_some_and(|y| y > 0.0),
            "the body moved: {:?}",
            offset(body)
        );
    }

    /// A program's piano is an id in the file and a name on the instrument. With nothing
    /// attached the document has the id and says as much; it never invents the name.
    #[test]
    fn a_pianos_name_comes_off_the_instrument_or_not_at_all() {
        use nord_usb::{Location, ObjectClass};

        let ctx = egui::Context::default();
        let mut workspace = Workspace::new(ctx.clone());
        let device = Device::new(ctx);
        let mut log = Log::default();

        let id = workspace.create(Fresh::Program, &mut log).unwrap();
        let bytes = workspace.get(id).unwrap().bytes.clone();
        let fields = fields::apply(&bytes, &[]).unwrap().0;
        let local = piano_lookup(workspace.get(id).unwrap(), Some(&fields), &device);
        assert!(local.name.is_none(), "nothing has been asked");
        assert!(!local.can_ask, "and there is nothing to ask");

        let from_device = workspace.ingest(
            "Africa-Split.ne5p".into(),
            Origin::Device {
                class: ObjectClass::Program,
                at: Location { bank: 6, slot: 3 },
            },
            bytes,
            &mut log,
        );
        let copied = piano_lookup(workspace.get(from_device).unwrap(), Some(&fields), &device);
        assert!(copied.name.is_none(), "still nothing has been asked");
        // A fresh program references no piano at all, and zero is not an id to hunt for.
        assert_eq!(copied.id, None);
    }

    /// The registry spells an id in decimal; a person spells it the way `nord deps` does.
    #[test]
    fn a_library_id_reads_in_either_spelling() {
        assert_eq!(library_id("16909060"), Some(0x0102_0304));
        assert_eq!(library_id("0x01020304"), Some(0x0102_0304));
        assert_eq!(library_id(" 0 "), Some(0));
        assert_eq!(library_id("nothing"), None);
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
