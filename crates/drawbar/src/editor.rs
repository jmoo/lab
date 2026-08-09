//! The field editor: one editor over every body that carries the generated registry.
//!
//! Field paths, values and refusals all come from `nord-format`, so a field becomes
//! editable by being declared and this cannot fall behind the library.
//!
//! Edits are **staged**. Nothing touches the workspace entity until Apply: the pending
//! `path = value` sets are replayed onto a fresh decode of the unedited bytes, and what
//! comes out is the preview — the field values after, and the bytes after. Revert is
//! just dropping the list.

use std::collections::HashMap;
use std::io::Cursor;
use std::ops::Range;

use eframe::egui;
use nord_format::cbin::Cbin;
use nord_format::fields::{Field, FieldError};
use nord_format::formats::{ne5, ns2, ns3};
use nord_format::{Entity, Live, Program, Settings};

use crate::app::{BAD, WARN};
use crate::drawbar_widget;
use crate::log::Log;
use crate::workspace::{Origin, Workspace};

/// The bodies this editor drives: one vocabulary — `path = value` — over each.
trait Editable {
    fn fields(&self) -> Vec<Field>;
    fn set_field(&mut self, path: &str, value: &str) -> Result<(), FieldError>;
}

macro_rules! editable {
    ($body:ty) => {
        impl Editable for Cbin<$body> {
            fn fields(&self) -> Vec<Field> {
                self.body.fields()
            }
            fn set_field(&mut self, path: &str, value: &str) -> Result<(), FieldError> {
                self.body.set_field(path, value)
            }
        }
    };
}

editable!(ne5::Program);
editable!(ne5::Settings);
editable!(ns2::Program);
editable!(ns3::Program);

/// The registry-backed body an entity holds, if it has one.
///
/// ⚠️ Kept in step with [`body_mut`] below — a variant in one and not the other is a
/// body that lists its fields and refuses to set them, or the reverse.
fn body(entity: &Entity) -> Option<&dyn Editable> {
    match entity {
        Entity::Program(Program::Electro5(f)) => Some(f),
        Entity::Program(Program::Stage2(f)) => Some(f),
        Entity::Program(Program::Stage3(f)) => Some(f),
        // The live buffer is the program body under another tag, so the fields are
        // identical.
        Entity::Live(Live::Electro5(f)) => Some(f),
        Entity::Live(Live::Stage2(f)) => Some(f),
        Entity::Live(Live::Stage3(f)) => Some(f),
        Entity::Settings(Settings::Electro5(f)) => Some(f),
        _ => None,
    }
}

fn body_mut(entity: &mut Entity) -> Option<&mut dyn Editable> {
    match entity {
        Entity::Program(Program::Electro5(f)) => Some(f),
        Entity::Program(Program::Stage2(f)) => Some(f),
        Entity::Program(Program::Stage3(f)) => Some(f),
        Entity::Live(Live::Electro5(f)) => Some(f),
        Entity::Live(Live::Stage2(f)) => Some(f),
        Entity::Live(Live::Stage3(f)) => Some(f),
        Entity::Settings(Settings::Electro5(f)) => Some(f),
        _ => None,
    }
}

/// Every registered field's current value, for a body that has a registry.
pub fn fields_of(entity: &Entity) -> Option<Vec<Field>> {
    body(entity).map(|body| body.fields())
}

pub fn can_edit(entity: &Entity) -> bool {
    body(entity).is_some()
}

/// ⚠️ Fields that do nothing without a companion. The pairing is a fact about the
/// instrument, not something the declaration carries.
///
/// Transpose: the stored value is ignored while `transpose_enabled` is clear, the
/// instrument never clears that bit once set, and an untouched program holds `+1` rather
/// than `0`. Setting one half deliberately is legitimate, so this warns rather than
/// refusing.
const STICKY_PAIRS: [(&str, &str); 1] =
    [("center_panel.transpose", "center_panel.transpose_enabled")];

/// Longest legal-value list that stays a menu. Past it, a run of consecutive integers is
/// a slider instead.
const CHOICE_MAX: usize = 24;

enum Widget {
    Bool,
    Choice,
    Number {
        min: i64,
        max: i64,
    },
    /// A nine-nibble organ register.
    Register,
    /// Too wide to enumerate, so the stored bits are its only spelling.
    Hex,
}

impl Widget {
    fn of(field: &Field, legal: &[String]) -> Widget {
        if drawbar_widget::is_register(&field.path, field.spec.width) {
            return Widget::Register;
        }
        // A field past the enumerable ceiling lists no values at all.
        if legal.is_empty() {
            return Widget::Hex;
        }
        if legal.len() == 2 && legal[0] == "false" && legal[1] == "true" {
            return Widget::Bool;
        }
        if legal.len() > CHOICE_MAX {
            if let Some((min, max)) = contiguous(legal) {
                return Widget::Number { min, max };
            }
        }
        Widget::Choice
    }
}

/// The range a legal-value list covers, when every value is an integer and none is
/// missing. A gapped set stays a menu — a slider over it would stop on values the field
/// refuses.
fn contiguous(legal: &[String]) -> Option<(i64, i64)> {
    let mut values = Vec::with_capacity(legal.len());
    for value in legal {
        values.push(value.trim_start_matches('+').parse::<i64>().ok()?);
    }
    let min = *values.iter().min()?;
    let max = *values.iter().max()?;
    (max.checked_sub(min)? + 1 == values.len() as i64).then_some((min, max))
}

/// The unedited entity, and everything about its fields that is worth computing once.
struct Base {
    bytes: Vec<u8>,
    fields: Vec<Field>,
    /// ⚠️ `FieldSpec::legal` walks every bit pattern the field can hold — thousands at
    /// the enumerable ceiling — so it is called here and never once a frame.
    legal: Vec<Vec<String>>,
    widget: Vec<Widget>,
}

struct Preview {
    fields: Vec<Field>,
    bytes: Vec<u8>,
}

#[derive(Default)]
pub struct Editor {
    target: Option<u64>,
    /// Pending sets, one entry per path.
    staged: Vec<(String, String)>,
    base: Option<Base>,
    preview: Option<Preview>,
    /// The last refusal, and the path that caused it.
    error: Option<(String, String)>,
    /// Text buffers for the fields typed rather than picked; a `TextEdit` needs
    /// somewhere to keep a half-typed value between frames.
    text: HashMap<String, String>,
}

impl Editor {
    /// Point the editor at a workspace entity, discarding anything staged for another.
    pub fn open(&mut self, id: u64) {
        if self.target != Some(id) {
            self.reset();
            self.target = Some(id);
        }
    }

    fn reset(&mut self) {
        self.staged.clear();
        self.base = None;
        self.preview = None;
        self.error = None;
        self.text.clear();
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, workspace: &mut Workspace, log: &mut Log) {
        let Some(entity) = workspace.selected() else {
            ui.label(
                egui::RichText::new("Select a workspace entity to edit it.")
                    .weak()
                    .italics(),
            );
            return;
        };
        let id = entity.id;
        let title = entity.name.clone();
        let from_device = matches!(entity.origin, Origin::Device { .. });
        let Some(decoded) = &entity.entity else {
            ui.label(egui::RichText::new("This entity did not decode.").color(BAD));
            return;
        };
        if !can_edit(decoded) {
            ui.label(
                egui::RichText::new(
                    "No field registry for this format — the inspector can still read it.",
                )
                .weak(),
            );
            return;
        }
        // Reopen whenever the selection moves, or the bytes changed underneath (an
        // apply, or a fresh read off the instrument).
        if self.target != Some(id) || self.base.as_ref().is_none_or(|b| b.bytes != entity.bytes) {
            self.reset();
            self.target = Some(id);
            self.base = Base::open(&entity.bytes);
        }

        let Some(base) = &self.base else {
            ui.label(egui::RichText::new("This entity has no field registry.").weak());
            return;
        };

        ui.horizontal_wrapped(|ui| {
            ui.heading(&title);
            ui.label(
                egui::RichText::new(format!("{} fields", base.fields.len()))
                    .small()
                    .weak(),
            );
        });
        if from_device {
            ui.label(
                egui::RichText::new(
                    "This is a copy in the workspace. Edit it here, then Put it back from \
                     the Instrument pane.",
                )
                .small()
                .weak(),
            );
        }

        let mut apply = false;
        let mut revert = false;
        ui.horizontal_wrapped(|ui| {
            let staged = !self.staged.is_empty();
            apply = ui
                .add_enabled(staged, egui::Button::new("Apply"))
                .on_hover_text("re-encode and replace the workspace entity's bytes")
                .clicked();
            revert = ui
                .add_enabled(staged, egui::Button::new("Revert"))
                .clicked();
            if staged {
                ui.label(egui::RichText::new(format!("{} staged", self.staged.len())).color(WARN));
            }
        });
        if let Some((path, why)) = &self.error {
            ui.label(egui::RichText::new(format!("{path}: {why}")).color(BAD));
        }

        let mut sets = Vec::new();
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                self.changes(ui);
                let base = self.base.as_ref().expect("checked above");
                let fields = match &self.preview {
                    Some(preview) => &preview.fields,
                    None => &base.fields,
                };
                let stale = stale_register(fields);
                render_fields(ui, base, fields, &mut self.text, stale, &mut sets);
            });

        for (path, value) in sets {
            self.stage(&path, value);
        }
        if revert {
            self.staged.clear();
            self.preview = None;
            self.error = None;
            self.text.clear();
            log.info("reverted the staged edits");
        }
        if apply {
            if let Some(preview) = self.preview.take() {
                let count = self.staged.len();
                workspace.replace_bytes(id, preview.bytes, log);
                self.reset();
                self.target = Some(id);
                log.info(format!("applied {count} field change(s) to {title}"));
            }
        }
    }

    /// The Changes panel: every field that moved, and the bytes that moved with it.
    fn changes(&self, ui: &mut egui::Ui) {
        let (Some(base), Some(preview)) = (&self.base, &self.preview) else {
            return;
        };
        egui::CollapsingHeader::new("changes")
            .default_open(true)
            .show(ui, |ui| {
                let staged: Vec<&str> = self.staged.iter().map(|(p, _)| p.as_str()).collect();
                egui::Grid::new("staged").num_columns(3).show(ui, |ui| {
                    for (before, after) in base.fields.iter().zip(&preview.fields) {
                        if before.display == after.display {
                            continue;
                        }
                        ui.label(egui::RichText::new(&after.path).monospace().small());
                        ui.label(egui::RichText::new(&before.display).weak());
                        ui.label(egui::RichText::new(&after.display).strong());
                        ui.end_row();
                    }
                });
                for (field, companion) in STICKY_PAIRS {
                    if staged.contains(&field) && !staged.contains(&companion) {
                        ui.label(
                            egui::RichText::new(format!(
                                "⚠ {field} was set but {companion} was not; the instrument \
                                 reads the pair, not either half alone",
                            ))
                            .small()
                            .color(WARN),
                        );
                    }
                }

                ui.separator();
                let diff = byte_diff(&base.bytes, &preview.bytes);
                if diff.is_empty() {
                    ui.label(egui::RichText::new("no bytes moved").weak().small());
                }
                for row in diff {
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

    /// Stage one set, or drop it if the library refuses the value.
    fn stage(&mut self, path: &str, value: String) {
        let original = self
            .base
            .as_ref()
            .and_then(|b| b.fields.iter().find(|f| f.path == path))
            .map(|f| f.value.clone());
        self.staged.retain(|(p, _)| p != path);
        // Back where it started is not a change.
        if Some(&value) != original.as_ref() {
            self.staged.push((path.to_string(), value));
        }
        self.error = None;
        if let Err(why) = self.recompute() {
            self.staged.retain(|(p, _)| p != path);
            self.error = Some((path.to_string(), why));
            let _ = self.recompute();
        }
    }

    fn recompute(&mut self) -> Result<(), String> {
        let Some(base) = &self.base else {
            return Ok(());
        };
        if self.staged.is_empty() {
            self.preview = None;
            return Ok(());
        }
        let (fields, bytes) = apply_all(&base.bytes, &self.staged)?;
        self.preview = Some(Preview { fields, bytes });
        Ok(())
    }
}

impl Base {
    fn open(bytes: &[u8]) -> Option<Base> {
        let entity = nord_format::from_stream(&mut Cursor::new(bytes)).ok()?;
        let fields = body(&entity)?.fields();
        let legal: Vec<Vec<String>> = fields.iter().map(|f| (f.spec.legal)()).collect();
        let widget = fields
            .iter()
            .zip(&legal)
            .map(|(field, legal)| Widget::of(field, legal))
            .collect();
        Some(Base {
            bytes: bytes.to_vec(),
            fields,
            legal,
            widget,
        })
    }
}

/// Replay every staged set onto a fresh decode of the unedited bytes.
///
/// Every change lands before anything is encoded, so a value the field cannot hold
/// cannot leave a half-edited body behind.
fn apply_all(bytes: &[u8], sets: &[(String, String)]) -> Result<(Vec<Field>, Vec<u8>), String> {
    let mut entity =
        nord_format::from_stream(&mut Cursor::new(bytes)).map_err(|e| e.to_string())?;
    {
        let body = body_mut(&mut entity).ok_or("this entity has no field registry")?;
        for (path, value) in sets {
            body.set_field(path, value).map_err(|e| e.to_string())?;
        }
    }
    let fields = body(&entity)
        .ok_or("this entity has no field registry")?
        .fields();
    let out = nord_format::to_bytes(&entity).map_err(|e| e.to_string())?;
    Ok((fields, out))
}

/// The register whose nine nibbles the instrument is not reading, if any.
///
/// ⚠️ In b3+bass, preset 1 is the bass manual: only two drawbars are live, and they sit
/// outside the nine-nibble block — which holds stale leftovers, not zeroes. Nine
/// draggable bars there would assert that moving them does something.
fn stale_register(fields: &[Field]) -> Option<&'static str> {
    let organ = fields
        .iter()
        .find(|f| f.path == "center_panel.organ_type")?;
    (organ.value == "B3Bass").then_some("organ_panel.b3_preset1_drawbars")
}

fn render_fields(
    ui: &mut egui::Ui,
    base: &Base,
    fields: &[Field],
    text: &mut HashMap<String, String>,
    stale: Option<&str>,
    out: &mut Vec<(String, String)>,
) {
    let paths: Vec<&str> = fields.iter().map(|f| f.path.as_str()).collect();
    for (group, members) in crate::inspect::group_paths(paths.iter().copied()) {
        let title = match group {
            "" => "body",
            name => name,
        };
        egui::CollapsingHeader::new(title)
            .id_salt(("edit", title))
            .show(ui, |ui| {
                for i in members {
                    let field = &fields[i];
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new(crate::inspect::leaf(&field.path))
                                .small()
                                .weak(),
                        )
                        .on_hover_text(field.spec.placement);
                        if let Some(value) =
                            widget_ui(ui, field, &base.legal[i], &base.widget[i], text, stale)
                        {
                            out.push((field.path.clone(), value));
                        }
                    });
                }
            });
    }
}

/// One field's widget. `Some` when the user moved it.
fn widget_ui(
    ui: &mut egui::Ui,
    field: &Field,
    legal: &[String],
    widget: &Widget,
    text: &mut HashMap<String, String>,
    stale: Option<&str>,
) -> Option<String> {
    match widget {
        Widget::Bool => {
            let mut on = field.value == "true";
            ui.checkbox(&mut on, "").changed().then(|| on.to_string())
        }
        Widget::Choice => {
            let mut picked = None;
            egui::ComboBox::from_id_salt(&field.path)
                .selected_text(&field.value)
                .width(150.0)
                .show_ui(ui, |ui| {
                    for value in legal {
                        if ui.selectable_label(*value == field.value, value).clicked() {
                            picked = Some(value.clone());
                        }
                    }
                });
            picked.filter(|value| *value != field.value)
        }
        Widget::Number { min, max } => {
            let mut value: i64 = field.value.trim_start_matches('+').parse().ok()?;
            let before = value;
            ui.add(egui::DragValue::new(&mut value).range(*min..=*max));
            (value != before).then(|| value.to_string())
        }
        Widget::Register => register_ui(ui, field, text, stale),
        Widget::Hex => hex_ui(ui, field, text),
    }
}

fn register_ui(
    ui: &mut egui::Ui,
    field: &Field,
    text: &mut HashMap<String, String>,
    stale: Option<&str>,
) -> Option<String> {
    let Some(bits) = drawbar_widget::parse(&field.value) else {
        // Not a value the nine-nibble shape can hold; the stored bits are still
        // editable, which is more use than a widget that cannot show it.
        return hex_ui(ui, field, text);
    };
    let live = stale != Some(field.path.as_str());
    let mut set = None;
    ui.vertical(|ui| {
        if !live {
            ui.label(
                egui::RichText::new(
                    "b3+bass: preset 1 is the bass manual. These nine nibbles are stale — \
                     the live pair is organ_panel.b3_bass_bar1 / bar2.",
                )
                .small()
                .color(WARN),
            );
        }
        if let Some(bars) = drawbar_widget::ui(ui, drawbar_widget::bars(bits), live) {
            set = Some(drawbar_widget::spell(drawbar_widget::bits(bars)));
        }
        // The stored bits stay reachable, so a value the bars cannot express — or a
        // deliberate edit of a stale block — is still possible.
        if let Some(typed) = hex_ui(ui, field, text) {
            set = Some(typed);
        }
    });
    set
}

/// A text box over a field's stored bits, committed on Enter or on losing focus.
fn hex_ui(ui: &mut egui::Ui, field: &Field, text: &mut HashMap<String, String>) -> Option<String> {
    let buffer = text
        .entry(field.path.clone())
        .or_insert_with(|| field.value.clone());
    let response = ui.add(
        egui::TextEdit::singleline(buffer)
            .desired_width(120.0)
            .font(egui::TextStyle::Monospace),
    );
    // Not committed per keystroke: a half-typed `0x8` is a legal value that would be
    // staged and then re-read as the field's new value mid-word.
    let done = response.lost_focus() || response.ctx.input(|i| i.key_pressed(egui::Key::Enter));
    if !done {
        return None;
    }
    let typed = buffer.trim().to_string();
    if typed == field.value {
        return None;
    }
    Some(typed)
}

/// One byte that moved.
pub struct DiffRow {
    pub at: usize,
    pub before: u8,
    pub after: u8,
    /// `  (body crc32)` where the byte is bookkeeping rather than an edit.
    pub note: &'static str,
}

/// Where a CBIN file keeps its checksum and what to call it, or `None` for bytes that
/// are not a CBIN file.
///
/// ⚠️ The two generations put it in different places, and a type-0 file's `0x18` is body
/// data — annotating it as the type-1 crc32 would label a real edit as bookkeeping.
fn checksum_bytes(file: &[u8]) -> Option<(Range<usize>, &'static str)> {
    if file.len() < 8 || &file[0..4] != nord_format::cbin::MAGIC {
        return None;
    }
    match u32::from_le_bytes(file[4..8].try_into().ok()?) {
        0 => Some((file.len() - 2..file.len(), "  (file crc16)")),
        1 => Some((0x18..0x1c, "  (body crc32)")),
        _ => None,
    }
}

/// The bytes that moved.
///
/// The checksum moves with any body change; those rows are annotated so they do not read
/// as a second unexplained edit. A length change is not a diff at all — nothing here can
/// pair the bytes up — so it comes back empty.
pub fn byte_diff(before: &[u8], after: &[u8]) -> Vec<DiffRow> {
    if before.len() != after.len() {
        return Vec::new();
    }
    let checksum = checksum_bytes(after);
    before
        .iter()
        .zip(after)
        .enumerate()
        .filter(|(_, (b, a))| b != a)
        .map(|(at, (&b, &a))| DiffRow {
            at,
            before: b,
            after: a,
            note: match &checksum {
                Some((range, label)) if range.contains(&at) => label,
                _ => "",
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program() -> Vec<u8> {
        let entity = Entity::Program(Program::Electro5(ne5::program::new(
            (0, 0).try_into().unwrap(),
        )));
        nord_format::to_bytes(&entity).unwrap()
    }

    #[test]
    fn a_staged_set_changes_the_field_it_names_and_nothing_else() {
        let bytes = program();
        let (before, _) = apply_all(&bytes, &[]).unwrap();
        let (after, edited) =
            apply_all(&bytes, &[("center_panel.gain".into(), "96".into())]).unwrap();

        let moved: Vec<&str> = before
            .iter()
            .zip(&after)
            .filter(|(b, a)| b.display != a.display)
            .map(|(_, a)| a.path.as_str())
            .collect();
        assert_eq!(moved, ["center_panel.gain"]);
        assert_eq!(edited.len(), bytes.len());
    }

    /// A value the field cannot hold is refused before anything is encoded, and the
    /// message names what it does accept.
    #[test]
    fn an_out_of_range_value_is_refused_by_the_library() {
        let err = apply_all(&program(), &[("center_panel.gain".into(), "200".into())])
            .err()
            .expect("200 is not a gain");
        assert!(err.contains("not a value of gain"), "{err}");
        assert!(err.contains("0 .. 127"), "{err}");
    }

    /// The crc moves with any body change; the row has to say so, or it reads as a
    /// second edit nobody made.
    #[test]
    fn the_checksum_bytes_are_annotated_as_bookkeeping() {
        let bytes = program();
        let (_, edited) = apply_all(&bytes, &[("center_panel.gain".into(), "96".into())]).unwrap();
        let diff = byte_diff(&bytes, &edited);
        assert!(!diff.is_empty());
        // A fresh program is type-1, so the crc32 sits at 0x18..0x1c.
        let annotated: Vec<usize> = diff
            .iter()
            .filter(|row| row.note.contains("crc32"))
            .map(|row| row.at)
            .collect();
        assert!(annotated.iter().all(|at| (0x18..0x1c).contains(at)));
        assert!(!annotated.is_empty(), "the crc32 must have moved");
        assert!(
            diff.iter().any(|row| row.note.is_empty()),
            "the edit itself must show as an unannotated byte",
        );
    }

    /// A gapped legal set must stay a menu: a slider over it would stop on values the
    /// field refuses.
    #[test]
    fn only_a_gapless_run_of_integers_becomes_a_slider() {
        let full: Vec<String> = (0..128).map(|n| n.to_string()).collect();
        assert_eq!(contiguous(&full), Some((0, 127)));

        let gapped: Vec<String> = vec!["0".into(), "1".into(), "9".into()];
        assert_eq!(contiguous(&gapped), None);

        let named: Vec<String> = vec!["Organ".into(), "Piano".into()];
        assert_eq!(contiguous(&named), None);
    }

    /// The nine-nibble register is the drawbar widget's, and nothing else is.
    #[test]
    fn a_register_field_picks_the_drawbar_widget() {
        let bytes = program();
        let (fields, _) = apply_all(&bytes, &[]).unwrap();
        let register = fields
            .iter()
            .find(|f| f.path == "organ_panel.b3_preset1_drawbars")
            .expect("a program has a b3 preset 1 register");
        assert!(matches!(
            Widget::of(register, &(register.spec.legal)()),
            Widget::Register
        ));

        let gain = fields
            .iter()
            .find(|f| f.path == "center_panel.gain")
            .expect("a program has a gain");
        assert!(matches!(
            Widget::of(gain, &(gain.spec.legal)()),
            Widget::Number { min: 0, max: 127 }
        ));
    }

    /// The stale-nibble case is detected off the selection the file stores, not guessed.
    #[test]
    fn b3_bass_marks_preset_one_as_stale() {
        let bytes = program();
        let (plain, _) = apply_all(&bytes, &[]).unwrap();
        assert_eq!(stale_register(&plain), None);

        let (bass, _) = apply_all(
            &bytes,
            &[("center_panel.organ_type".into(), "B3Bass".into())],
        )
        .unwrap();
        assert_eq!(
            stale_register(&bass),
            Some("organ_panel.b3_preset1_drawbars")
        );
    }

    /// A drawbar pulled in the widget writes back exactly what the field reads out, so
    /// the round trip through the registry is a no-op when nothing moved.
    #[test]
    fn a_register_round_trips_through_the_widgets_spelling() {
        let bytes = program();
        let (fields, _) = apply_all(&bytes, &[]).unwrap();
        let register = fields
            .iter()
            .find(|f| f.path == "organ_panel.vox_preset1_drawbars")
            .unwrap();
        let bits = drawbar_widget::parse(&register.value).unwrap();
        let spelled = drawbar_widget::spell(drawbar_widget::bits(drawbar_widget::bars(bits)));
        assert_eq!(spelled, register.value);

        let (after, _) =
            apply_all(&bytes, &[(register.path.clone(), "0x888800000".into())]).unwrap();
        let edited = after.iter().find(|f| f.path == register.path).unwrap();
        assert_eq!(
            drawbar_widget::bars(drawbar_widget::parse(&edited.value).unwrap()),
            [8, 8, 8, 8, 0, 0, 0, 0, 0]
        );
    }
}
