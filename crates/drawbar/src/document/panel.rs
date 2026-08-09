//! The Electro 5 panel as a document: the sections the instrument itself is divided
//! into, holding the controls that instrument would be showing.

use eframe::egui;
use nord_format::fields::Field;

use super::controls::{self, Ctx, Sets};
use crate::app::{GOOD, WARN};
use crate::drawbar_widget;
use crate::fields::Control;
use crate::strings::{self, Section};
use crate::visibility::{self, Bars, Organ, Registration};

/// A program or a live slot: the same body, so the same panel.
pub fn program(ui: &mut egui::Ui, ctx: &Ctx, fields: &[Field], sets: &mut Sets) {
    let organ = visibility::organ(fields);
    for section in strings::PROGRAM_SECTIONS {
        if !visibility::shown(section, fields) {
            continue;
        }
        let mut rows = gather(fields, section, organ.as_ref());
        if visibility::switches_first(section) {
            reading_order(ctx, &mut rows);
        }
        let organ_here = (section == Section::Organ)
            .then_some(organ.as_ref())
            .flatten();
        if rows.is_empty() && organ_here.is_none() {
            continue;
        }
        controls::section(ui, section.title(), |ui| {
            if section == Section::Keyboard {
                transpose(ui, fields, sets);
            }
            match organ_here {
                Some(organ) => organ_section(ui, ctx, fields, organ, &rows, sets),
                None => {
                    for field in &rows {
                        controls::row(ui, ctx, field, sets);
                    }
                }
            }
        });
    }
}

/// The settings body, in the order the instrument's own menus run.
pub fn settings(ui: &mut egui::Ui, ctx: &Ctx, fields: &[Field], sets: &mut Sets) {
    for section in strings::SETTINGS_SECTIONS {
        let rows = gather(fields, section, None);
        if rows.is_empty() {
            continue;
        }
        controls::section(ui, section.title(), |ui| {
            for field in &rows {
                controls::row(ui, ctx, field, sets);
            }
        });
    }
}

/// Any other registry-backed body: one flat list, because nothing here knows how that
/// instrument's panel is divided.
pub fn plain(ui: &mut egui::Ui, ctx: &Ctx, fields: &[Field], sets: &mut Sets) {
    ui.label(
        egui::RichText::new("Every field this format declares.")
            .small()
            .weak(),
    );
    for field in fields {
        controls::row(ui, ctx, field, sets);
    }
}

/// The fields a section shows: its own, minus what another control speaks for.
fn gather<'a>(fields: &'a [Field], section: Section, organ: Option<&Organ>) -> Vec<&'a Field> {
    fields
        .iter()
        .filter(|field| strings::section(&field.path) == section)
        .filter(|field| !visibility::engineering_only(&field.path))
        .filter(|field| !organ.is_some_and(|organ| organ.covers(&field.path)))
        .collect()
}

/// One effect at a time, and within an effect what it *is* before how much of it there
/// is — the order the panel's own labelling reads in.
fn reading_order(ctx: &Ctx, rows: &mut [&Field]) {
    let group = |path: &str| -> String {
        let leaf = path.rsplit('.').next().unwrap_or(path);
        leaf.split_once('_')
            .map_or(leaf, |(head, _)| head)
            .to_string()
    };
    let mut order: Vec<String> = Vec::new();
    for field in rows.iter() {
        let key = group(&field.path);
        if !order.contains(&key) {
            order.push(key);
        }
    }
    rows.sort_by_key(|field| {
        let at = order
            .iter()
            .position(|key| *key == group(&field.path))
            .unwrap_or(usize::MAX);
        let knob = !matches!(
            ctx.control.get(&field.path),
            Some(Control::Toggle) | Some(Control::Choice)
        );
        (at, knob)
    });
}

fn find<'a>(fields: &'a [Field], path: &str) -> Option<&'a Field> {
    fields.iter().find(|field| field.path == path)
}

fn organ_section(
    ui: &mut egui::Ui,
    ctx: &Ctx,
    fields: &[Field],
    organ: &Organ,
    rows: &[&Field],
    sets: &mut Sets,
) {
    // The model picker is always here, whatever it is set to: it is how a program comes
    // to have one organ rather than another.
    if let Some(field) = find(fields, "center_panel.organ_type") {
        controls::row(ui, ctx, field, sets);
    }
    if !organ.known {
        ui.label(
            egui::RichText::new(
                "This program has an organ selection this app does not recognise, so it \
                 cannot say which registration the instrument is playing. Everything the \
                 file stores is below.",
            )
            .small()
            .color(WARN),
        );
    }

    for path in organ.vib_type.iter().chain(
        organ
            .perc
            .iter()
            .flat_map(|(third, speed)| [third, speed].into_iter()),
    ) {
        if let Some(field) = find(fields, path) {
            controls::row(ui, ctx, field, sets);
        }
    }

    for registration in &organ.registrations {
        preset(ui, fields, organ, registration, sets);
    }

    // Whatever else this section holds: an unrecognised selection's whole panel, or a
    // field the library has grown since.
    for field in rows {
        if field.path == "center_panel.organ_type" {
            continue;
        }
        controls::row(ui, ctx, field, sets);
    }
}

fn preset(
    ui: &mut egui::Ui,
    fields: &[Field],
    organ: &Organ,
    registration: &Registration,
    sets: &mut Sets,
) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let title = match &registration.bars {
                Bars::Bass(..) => format!("Preset {} — bass manual", registration.preset),
                _ => format!("Preset {}", registration.preset),
            };
            let picked = ui
                .selectable_label(registration.live, egui::RichText::new(title).strong())
                .on_hover_text("the preset the instrument plays");
            if picked.clicked() && !registration.live {
                if let Some(path) = organ.preset_field {
                    sets.push((path.to_string(), (registration.preset == 2).to_string()));
                }
            }
            if registration.live {
                ui.label(egui::RichText::new("playing").small().color(GOOD));
            }
        });

        match &registration.bars {
            Bars::Nine(path) => nine(ui, fields, path, sets),
            Bars::Tabs(path) => {
                nine(ui, fields, path, sets);
                ui.label(
                    egui::RichText::new("The instrument reads a register at 5 or more as on.")
                        .small()
                        .weak(),
                );
            }
            Bars::Bass(first, second) => bass(ui, fields, first, second, sets),
        }

        ui.horizontal_wrapped(|ui| {
            controls::switch(
                ui,
                registration.vib.and_then(|p| find(fields, p)),
                "vibrato",
                sets,
            );
            controls::switch(
                ui,
                registration.perc.and_then(|p| find(fields, p)),
                "percussion",
                sets,
            );
        });
    });
}

fn nine(ui: &mut egui::Ui, fields: &[Field], path: &str, sets: &mut Sets) {
    let Some(field) = find(fields, path) else {
        return;
    };
    if let Some(value) = controls::register(ui, field, true) {
        sets.push((field.path.clone(), value));
    }
}

/// The bass manual: two drawbars, each its own field, written together so a pull moves
/// the registration rather than half of it.
fn bass(ui: &mut egui::Ui, fields: &[Field], first: &str, second: &str, sets: &mut Sets) {
    let read = |path: &str| -> u8 {
        find(fields, path)
            .and_then(|field| field.value.parse().ok())
            .unwrap_or(0)
    };
    let mut positions = [0u8; drawbar_widget::BARS];
    positions[0] = read(first);
    positions[1] = read(second);
    if let Some(moved) = controls::bars(ui, positions, true, 2) {
        sets.push((first.to_string(), moved[0].to_string()));
        sets.push((second.to_string(), moved[1].to_string()));
    }
}

/// The transpose control: one switch and one number, written together the way the
/// panel's own button writes them.
pub fn transpose(ui: &mut egui::Ui, fields: &[Field], sets: &mut Sets) {
    let Some((on, semitones)) = visibility::transpose(fields) else {
        return;
    };
    ui.horizontal(|ui| {
        controls::label(ui, "center_panel.transpose");
        let mut want_on = on;
        let mut want = semitones;
        let switched = ui
            .checkbox(&mut want_on, "")
            .on_hover_text("the transpose light on the panel")
            .changed();
        let sign = match want >= 0 {
            true => "+",
            false => "",
        };
        let moved = ui
            .add(egui::DragValue::new(&mut want).range(-6..=6).prefix(sign))
            .changed();
        if switched || moved {
            // Moving the semitones turns the light on, which is what the panel does.
            sets.extend(visibility::set_transpose(want_on || moved, want));
        }
        if !want_on {
            ui.label(
                egui::RichText::new("off — the instrument ignores the amount")
                    .small()
                    .weak(),
            );
        }
    });
}
