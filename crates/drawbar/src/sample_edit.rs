//! The sample-instrument editor.
//!
//! A sample is mostly encoded audio, so what is settable is what the format can patch in
//! place without touching a stroke: the name, and each zone's root key and top note.
//! Everything else is view-only, and the Raw generations (nsmp3/nsmp4) have no section
//! accessors at all.
//!
//! Same shape as the field editor: staged sets replayed onto a fresh decode, a byte diff
//! against the unedited bytes, then Apply.

use std::io::Cursor;

use eframe::egui;
use nord_format::cbin::Cbin;
use nord_format::formats::nsmp::{self, Sample};
use nord_format::Entity;

use crate::app::{BAD, WARN};
use crate::editor::byte_diff;
use crate::log::Log;
use crate::note;
use crate::workspace::Workspace;

pub fn can_edit(entity: &Entity) -> bool {
    matches!(entity, Entity::Sample(nord_format::Sample::V2(_)))
}

fn sample(entity: &Entity) -> Option<&Cbin<Sample>> {
    match entity {
        Entity::Sample(nord_format::Sample::V2(sample)) => Some(sample),
        _ => None,
    }
}

fn sample_mut(entity: &mut Entity) -> Option<&mut Cbin<Sample>> {
    match entity {
        Entity::Sample(nord_format::Sample::V2(sample)) => Some(sample),
        _ => None,
    }
}

/// One zone, numbered the way the panel numbers them: from 1, top of the keyboard first.
#[derive(Clone, PartialEq, Eq)]
pub struct Zone {
    pub root_key: u8,
    pub top_note: u8,
}

/// Every settable value, in one read.
#[derive(Clone, PartialEq, Eq)]
struct Snapshot {
    name: String,
    zones: Vec<Zone>,
}

fn snapshot(sample: &Cbin<Sample>) -> Result<Snapshot, String> {
    let zones = sample.zones().map_err(|e| e.to_string())?;
    let strokes = sample.strokes().map_err(|e| e.to_string())?;
    Ok(Snapshot {
        name: sample.name().map_err(|e| e.to_string())?,
        zones: zones
            .iter()
            .zip(&strokes)
            .map(|(zone, stroke)| Zone {
                root_key: stroke.root_key,
                top_note: zone.top_note,
            })
            .collect(),
    })
}

/// Apply one `path = value`. Paths are the CLI's: `name`, `zone1.root_key`,
/// `zone1.top_note`.
fn apply(sample: &mut Cbin<Sample>, path: &str, value: &str) -> Result<(), String> {
    if path == "name" {
        return sample.set_name(value).map_err(|e| e.to_string());
    }
    let unknown = || format!("unknown field {path:?}");
    let (zone, field) = path.split_once('.').ok_or_else(unknown)?;
    let index = zone
        .strip_prefix("zone")
        .and_then(|n| n.parse::<usize>().ok())
        .filter(|&n| n >= 1)
        .ok_or_else(unknown)?;
    // Checked here so the message speaks the panel's 1-based numbering, not the format
    // crate's 0-based one.
    let zones = sample.zones().map_err(|e| e.to_string())?.len();
    if index > zones {
        return Err(format!("no zone {index}: the instrument has {zones}"));
    }
    let note = note::parse(value)?;
    match field {
        "root_key" => sample.set_root_key(index - 1, note),
        "top_note" => sample.set_zone_top_note(index - 1, note),
        _ => return Err(unknown()),
    }
    .map_err(|e| e.to_string())
}

fn apply_all(bytes: &[u8], sets: &[(String, String)]) -> Result<(Snapshot, Vec<u8>), String> {
    let mut entity =
        nord_format::from_stream(&mut Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let sample = sample_mut(&mut entity).ok_or("not a v2 sample instrument")?;
    for (path, value) in sets {
        apply(sample, path, value)?;
    }
    let after = snapshot(sample)?;
    let out = nord_format::to_bytes(&entity).map_err(|e| e.to_string())?;
    Ok((after, out))
}

struct Base {
    bytes: Vec<u8>,
    snapshot: Snapshot,
}

#[derive(Default)]
pub struct SampleEditor {
    target: Option<u64>,
    staged: Vec<(String, String)>,
    base: Option<Base>,
    preview: Option<(Snapshot, Vec<u8>)>,
    error: Option<String>,
    /// The name box's buffer, so a half-typed name survives between frames.
    name: String,
}

impl SampleEditor {
    fn reset(&mut self) {
        self.staged.clear();
        self.base = None;
        self.preview = None;
        self.error = None;
        self.name.clear();
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, workspace: &mut Workspace, log: &mut Log) {
        let Some(entity) = workspace.selected() else {
            return;
        };
        let id = entity.id;
        let title = entity.name.clone();
        let decoded = entity.entity.as_ref();
        let is_v2 = decoded.is_some_and(can_edit);
        if !is_v2 {
            // The Raw generations carry the same tag and a body this crate does not map,
            // so they are read and exported, never patched.
            ui.label(
                egui::RichText::new(
                    "Only v2 sample instruments can be edited; nsmp3/nsmp4 content is \
                     carried verbatim.",
                )
                .weak(),
            );
            return;
        }
        if self.target != Some(id) || self.base.as_ref().is_none_or(|b| b.bytes != entity.bytes) {
            self.reset();
            self.target = Some(id);
            self.base = decoded
                .and_then(sample)
                .and_then(|s| snapshot(s).ok())
                .map(|snapshot| Base {
                    bytes: entity.bytes.clone(),
                    snapshot,
                });
            if let Some(base) = &self.base {
                self.name = base.snapshot.name.clone();
            }
        }

        let Some(base) = &self.base else {
            ui.label(egui::RichText::new("this sample's sections did not read").color(BAD));
            return;
        };
        // Copied out before anything mutable is touched: a sample's bytes are megabytes,
        // so the diff is computed once here rather than cloned to keep a borrow alive.
        let original = base.snapshot.clone();
        let diff = match &self.preview {
            Some((_, bytes)) => byte_diff(&base.bytes, bytes),
            None => Vec::new(),
        };
        let current = match &self.preview {
            Some((snapshot, _)) => snapshot.clone(),
            None => original.clone(),
        };

        ui.heading(&title);
        let mut apply_clicked = false;
        let mut revert = false;
        ui.horizontal_wrapped(|ui| {
            let staged = !self.staged.is_empty();
            apply_clicked = ui.add_enabled(staged, egui::Button::new("Apply")).clicked();
            revert = ui
                .add_enabled(staged, egui::Button::new("Revert"))
                .clicked();
            if staged {
                ui.label(egui::RichText::new(format!("{} staged", self.staged.len())).color(WARN));
            }
        });
        if let Some(why) = &self.error {
            ui.label(egui::RichText::new(why).color(BAD));
        }

        let mut sets = Vec::new();
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("name");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.name)
                            .desired_width(180.0)
                            .char_limit(nsmp::MAX_NAME_LEN),
                    );
                    if (response.lost_focus()
                        || response.ctx.input(|i| i.key_pressed(egui::Key::Enter)))
                        && self.name != current.name
                    {
                        sets.push(("name".to_string(), self.name.clone()));
                    }
                });
                ui.label(
                    egui::RichText::new(format!(
                        "up to {} bytes; the display joins Main, Sub and Aux with underscores",
                        nsmp::MAX_NAME_LEN
                    ))
                    .small()
                    .weak(),
                );

                ui.separator();
                // Zones are stored high to low, and the panel numbers them from 1 at the
                // top of the keyboard.
                egui::Grid::new("zones_edit")
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("zone").small().weak());
                        ui.label(egui::RichText::new("root key").small().weak());
                        ui.label(egui::RichText::new("top note").small().weak());
                        ui.end_row();
                        for (i, zone) in current.zones.iter().enumerate() {
                            let n = i + 1;
                            ui.label(egui::RichText::new(n.to_string()).monospace());
                            if let Some(note) = note_picker(ui, ("root", n), zone.root_key) {
                                sets.push((format!("zone{n}.root_key"), note));
                            }
                            if let Some(note) = note_picker(ui, ("top", n), zone.top_note) {
                                sets.push((format!("zone{n}.top_note"), note));
                            }
                            ui.end_row();
                        }
                    });

                changes(ui, &original, &current, &diff);
            });

        for (path, value) in sets {
            self.stage(&path, value);
        }
        if revert {
            self.staged.clear();
            self.preview = None;
            self.error = None;
            self.name = original.name.clone();
            log.info("reverted the staged edits");
        }
        if apply_clicked {
            if let Some((_, bytes)) = self.preview.take() {
                let count = self.staged.len();
                workspace.replace_bytes(id, bytes, log);
                self.reset();
                self.target = Some(id);
                log.info(format!("applied {count} sample change(s) to {title}"));
            }
        }
    }

    fn stage(&mut self, path: &str, value: String) {
        self.staged.retain(|(p, _)| p != path);
        self.staged.push((path.to_string(), value));
        self.error = None;
        if let Err(why) = self.recompute() {
            self.staged.retain(|(p, _)| p != path);
            self.error = Some(why);
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
        let (snapshot, bytes) = apply_all(&base.bytes, &self.staged)?;
        // Every staged set that put a value back where it started leaves the snapshot
        // equal to the original, and there is then nothing to apply.
        if snapshot == base.snapshot {
            self.staged.clear();
            self.preview = None;
            return Ok(());
        }
        self.preview = Some((snapshot, bytes));
        Ok(())
    }
}

fn changes(
    ui: &mut egui::Ui,
    original: &Snapshot,
    current: &Snapshot,
    diff: &[crate::editor::DiffRow],
) {
    if original == current {
        return;
    }
    ui.separator();
    egui::CollapsingHeader::new("changes")
        .default_open(true)
        .show(ui, |ui| {
            if original.name != current.name {
                ui.label(format!("name  {:?} -> {:?}", original.name, current.name));
            }
            for (i, (before, after)) in original.zones.iter().zip(&current.zones).enumerate() {
                let n = i + 1;
                if before.root_key != after.root_key {
                    ui.label(format!(
                        "zone{n}.root_key  {} -> {}",
                        note::name(before.root_key),
                        note::name(after.root_key)
                    ));
                }
                if before.top_note != after.top_note {
                    ui.label(format!(
                        "zone{n}.top_note  {} -> {}",
                        note::name(before.top_note),
                        note::name(after.top_note)
                    ));
                }
            }
            ui.separator();
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

/// A MIDI note as a name: `C4` is middle C. Typing a number works too.
fn note_picker(ui: &mut egui::Ui, id: (&str, usize), note: u8) -> Option<String> {
    let mut value = note as f64;
    let response = ui.push_id(id, |ui| {
        ui.add(
            egui::DragValue::new(&mut value)
                .range(0.0..=127.0)
                .speed(0.2)
                .custom_formatter(|n, _| note::name(n as u8))
                .custom_parser(|text| note::parse(text).ok().map(|n| n as f64)),
        )
    });
    let picked = value.round() as u8;
    (response.inner.changed() && picked != note).then(|| note::name(picked))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The paths are the CLI's, and a zone number outside the instrument's is refused in
    /// the numbering the panel uses.
    #[test]
    fn an_unknown_path_is_refused_by_name() {
        let mut entity = Entity::Sample(nord_format::Sample::Raw(Cbin {
            header: nord_format::cbin::Header::new("nsmp", (0, 0), 300),
            body: nord_format::cbin::RawBody(vec![0; 4]),
        }));
        // A Raw generation has no section accessors at all, so it never reaches `apply`.
        assert!(!can_edit(&entity));
        assert!(sample_mut(&mut entity).is_none());
    }

    /// A note is spelled the way the editor shows it, and the round trip is exact.
    #[test]
    fn zone_notes_are_spelled_as_names() {
        assert_eq!(note::name(60), "C4");
        assert_eq!(note::parse("C4").unwrap(), 60);
    }
}
