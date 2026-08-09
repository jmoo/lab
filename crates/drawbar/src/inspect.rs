//! The inspector: everything the app can say about one selected entity.
//!
//! The field table comes from `nord-format`'s generated registry rather than a
//! handwritten summary, so a field appears here by being declared and the view cannot
//! fall behind the library.

use eframe::egui;
use nord_format::{Bundle, Entity, Sample, Song};

use crate::workspace::LocalEntity;

#[derive(Default)]
pub struct Inspector {
    show_raw: bool,
    /// The entity the cached dump belongs to.
    ///
    /// ⚠️ `{:#?}` over an undecoded body prints every byte, and a piano library is
    /// hundreds of megabytes — it is rendered once and kept, never per frame.
    raw_for: Option<u64>,
    raw: String,
}

impl Inspector {
    /// Returns true when the reader asked to edit what they are looking at.
    pub fn ui(&mut self, ui: &mut egui::Ui, entity: Option<&LocalEntity>) -> bool {
        let Some(entity) = entity else {
            ui.label(
                egui::RichText::new("Nothing selected. Open a file to inspect it.")
                    .weak()
                    .italics(),
            );
            return false;
        };

        let mut edit = false;
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                edit = self.identity(ui, entity);
                container(ui, entity);
                if let Some(decoded) = &entity.entity {
                    fields(ui, decoded);
                    extras(ui, decoded);
                }
                self.raw_debug(ui, entity);
            });
        edit
    }

    fn identity(&mut self, ui: &mut egui::Ui, entity: &LocalEntity) -> bool {
        let mut edit = false;
        ui.horizontal_wrapped(|ui| {
            ui.heading(&entity.name);
            ui.label(egui::RichText::new(entity.tag()).monospace().weak());
            let editable = entity
                .entity
                .as_ref()
                .is_some_and(|e| crate::editor::can_edit(e) || crate::sample_edit::can_edit(e));
            if editable && ui.button("Edit").clicked() {
                edit = true;
            }
        });
        match &entity.entity {
            Some(decoded) => {
                ui.label(decoded.identity().kind);
            }
            None => {
                ui.label(
                    egui::RichText::new(entity.parse_error.as_deref().unwrap_or("did not decode"))
                        .color(crate::app::BAD),
                );
            }
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("verify");
            ui.label(
                egui::RichText::new(entity.verify.badge())
                    .strong()
                    .color(entity.verify.color()),
            );
            ui.label(egui::RichText::new(entity.verify.detail()).weak());
        });
        ui.separator();
        edit
    }

    fn raw_debug(&mut self, ui: &mut egui::Ui, entity: &LocalEntity) {
        let Some(decoded) = &entity.entity else {
            return;
        };
        ui.separator();
        ui.checkbox(&mut self.show_raw, "Raw debug");
        if !self.show_raw {
            return;
        }
        if self.raw_for != Some(entity.id) {
            self.raw = format!("{decoded:#?}");
            self.raw_for = Some(entity.id);
        }
        egui::ScrollArea::both()
            .max_height(360.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.label(egui::RichText::new(&self.raw).monospace().small());
            });
    }
}

/// `label   value`, the inspector's one row shape.
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
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("cbin").num_columns(2).show(ui, |ui| {
                row(
                    ui,
                    "generation",
                    format!("{:?}", container.header.generation),
                );
                row(ui, "format", container.tag());
                row(ui, "version", container.header.version.to_string());
                row(ui, "slot", slot(&container.header));
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
fn slot(header: &nord_format::cbin::Header) -> String {
    match header.slot() {
        (0xffff, 0xffff) => "none (a library file, not a slot save)".into(),
        (bank, slot) => format!("{}:{}", bank + 1, slot + 1),
    }
}

fn fields(ui: &mut egui::Ui, entity: &Entity) {
    // The one list of registry-backed bodies lives with the editor: a body this can
    // table is exactly a body that can be edited.
    let Some(fields) = crate::editor::fields_of(entity) else {
        return;
    };
    let paths: Vec<&str> = fields.iter().map(|f| f.path.as_str()).collect();
    for (group, members) in group_paths(paths.iter().copied()) {
        let title = match group {
            "" => "body",
            name => name,
        };
        egui::CollapsingHeader::new(title)
            .id_salt(title)
            .show(ui, |ui| {
                egui::Grid::new(title)
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        for i in members {
                            let field = &fields[i];
                            ui.label(leaf(&field.path));
                            ui.label(egui::RichText::new(&field.display).monospace());
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
}

/// Field indices grouped by the path segment before the first `.`, in first-seen
/// order. A path with no `.` groups under the empty name.
pub(crate) fn group_paths<'a>(paths: impl Iterator<Item = &'a str>) -> Vec<(&'a str, Vec<usize>)> {
    let mut groups: Vec<(&'a str, Vec<usize>)> = Vec::new();
    for (i, path) in paths.enumerate() {
        let head = path.split_once('.').map_or("", |(head, _)| head);
        match groups.iter_mut().find(|(name, _)| *name == head) {
            Some((_, members)) => members.push(i),
            None => groups.push((head, vec![i])),
        }
    }
    groups
}

/// The part of a path below its group.
pub(crate) fn leaf(path: &str) -> &str {
    path.split_once('.').map_or(path, |(_, rest)| rest)
}

/// What a format says beyond its field registry.
fn extras(ui: &mut egui::Ui, entity: &Entity) {
    match entity {
        Entity::Sample(Sample::V2(sample)) => {
            egui::CollapsingHeader::new("sample instrument")
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("sample").num_columns(2).show(ui, |ui| {
                        row(ui, "name", result(sample.name()));
                        row(ui, "categories", sample.categories().join(", "));
                    });
                    match (sample.zones(), sample.strokes()) {
                        (Ok(zones), Ok(strokes)) => {
                            // Zones are stored high to low, and the panel numbers them
                            // from 1 starting at the top of the keyboard.
                            egui::Grid::new("zones")
                                .num_columns(4)
                                .striped(true)
                                .show(ui, |ui| {
                                    row_head(ui, ["zone", "top note", "root key", "packets"]);
                                    for (i, zone) in zones.iter().enumerate() {
                                        ui.label(format!("{}", i + 1));
                                        ui.label(
                                            egui::RichText::new(crate::note::name(zone.top_note))
                                                .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(match strokes.get(i) {
                                                Some(s) => crate::note::name(s.root_key),
                                                None => "—".into(),
                                            })
                                            .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(match strokes.get(i) {
                                                Some(s) => s.packets.to_string(),
                                                None => "—".into(),
                                            })
                                            .monospace(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        }
                        (zones, strokes) => {
                            for e in [zones.err(), strokes.err()].into_iter().flatten() {
                                ui.label(egui::RichText::new(e.to_string()).color(crate::app::BAD));
                            }
                        }
                    }
                });
        }
        Entity::Song(Song::Electro5(song)) => {
            egui::CollapsingHeader::new("set list")
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("song").num_columns(2).show(ui, |ui| {
                        for (i, at) in song.body.programs().iter().enumerate() {
                            let (bank, slot) = at.inner();
                            row(
                                ui,
                                &format!("slot {}", i + 1),
                                format!("program {}:{}", bank + 1, slot + 1),
                            );
                        }
                    });
                });
        }
        Entity::Bundle(bundle) => bundle_extras(ui, bundle),
        _ => {}
    }
}

fn bundle_extras(ui: &mut egui::Ui, bundle: &Bundle) {
    egui::CollapsingHeader::new("bundle members")
        .default_open(true)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "A bundle is a ZIP of other files; it is read, not re-encoded.",
                )
                .weak(),
            );
            egui::Grid::new("bundle")
                .num_columns(2)
                .show(ui, |ui| match bundle {
                    Bundle::Drum2Bank(bank) => {
                        row(ui, "drum 2 programs", bank.programs.len().to_string());
                    }
                    Bundle::Drum3KitBank(bank) => {
                        row(ui, "drum 3P kits", bank.kits.len().to_string());
                    }
                    Bundle::Electro5(bundle) => {
                        row(ui, "name", bundle.name().unwrap_or_else(|| "none".into()));
                        row(ui, "pianos", bundle.pianos().len().to_string());
                        row(ui, "samples", bundle.samples().len().to_string());
                        row(ui, "not placed", bundle.skipped().len().to_string());
                    }
                });
            if let Bundle::Electro5(bundle) = bundle {
                for (member, why) in bundle.skipped() {
                    ui.label(
                        egui::RichText::new(format!("{member}: {why}"))
                            .small()
                            .color(crate::app::WARN),
                    );
                }
            }
        });
}

fn row_head<const N: usize>(ui: &mut egui::Ui, labels: [&str; N]) {
    for label in labels {
        ui.label(egui::RichText::new(label).weak().small());
    }
    ui.end_row();
}

fn result<T: std::fmt::Display, E: std::fmt::Display>(r: Result<T, E>) -> String {
    match r {
        Ok(v) => v.to_string(),
        Err(e) => format!("({e})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nord_format::formats::ne5;

    #[test]
    fn paths_group_under_the_segment_before_the_first_dot() {
        let paths = [
            "center_panel.gain",
            "organ_panel.model",
            "center_panel.split",
            "program_version",
            "organ_panel.b3_preset1",
        ];
        assert_eq!(
            group_paths(paths.into_iter()),
            vec![
                ("center_panel", vec![0, 2]),
                ("organ_panel", vec![1, 4]),
                ("", vec![3]),
            ],
        );
    }

    /// A deeper path keeps everything below the group in its leaf, so two fields of
    /// one nested body stay distinguishable.
    #[test]
    fn a_leaf_is_everything_below_the_group() {
        assert_eq!(leaf("effects_panel.fx1_rate"), "fx1_rate");
        assert_eq!(leaf("organ_panel.b3.preset1"), "b3.preset1");
        assert_eq!(leaf("program_version"), "program_version");
    }

    /// Every registry-backed body groups into named sections rather than one flat
    /// list — the whole point of the table.
    #[test]
    fn a_fresh_program_groups_into_its_panels() {
        let program = ne5::program::new((0, 0).try_into().unwrap());
        let entity = Entity::Program(nord_format::Program::Electro5(program));
        let fields = crate::editor::fields_of(&entity).expect("a program is registry-backed");
        let paths: Vec<&str> = fields.iter().map(|f| f.path.as_str()).collect();
        let groups: Vec<&str> = group_paths(paths.into_iter())
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        for panel in ["center_panel", "organ_panel", "effects_panel"] {
            assert!(groups.contains(&panel), "{panel} missing from {groups:?}");
        }
    }
}
