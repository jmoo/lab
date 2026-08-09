//! The pieces a panel is built out of: a titled section, a run of controls across it,
//! and one control with its name printed underneath.
//!
//! A control reads its value out of the field list and hands back a `path = value` set
//! when the user moves it. Nothing here writes: the document collects every set the frame
//! produced and applies them together, so a control that owns two fields moves both or
//! neither.
//!
//! The arrangement is the instrument's, not a form's. Names sit **under** what they name,
//! controls run left to right in strips and wrap when the window is narrow, and a value
//! is turned or lit rather than typed — the panel has no text boxes, so neither does
//! this, until you ask one for a number.

use std::collections::HashMap;

use eframe::egui;
use nord_format::fields::Field;

use crate::fields::Control;
use crate::{drawbar_widget, knob, led, strings, visibility};

/// What every row needs and none of them should compute twice.
pub struct Ctx {
    /// ⚠️ `FieldSpec::legal` walks every bit pattern a field can hold — thousands at the
    /// enumerable ceiling — so it is read once per document and never per frame.
    pub legal: HashMap<String, Vec<String>>,
    pub control: HashMap<String, Control>,
}

impl Ctx {
    pub fn read(fields: &[Field]) -> Ctx {
        let mut legal = HashMap::new();
        let mut control = HashMap::new();
        for field in fields {
            let values = (field.spec.legal)();
            control.insert(field.path.clone(), Control::of(field, &values));
            legal.insert(field.path.clone(), values);
        }
        Ctx { legal, control }
    }

    fn legal_of(&self, path: &str) -> &[String] {
        self.legal.get(path).map_or(&[], Vec::as_slice)
    }
}

/// Collected `path = value` sets, applied together once the frame is painted.
pub type Sets = Vec<(String, String)>;

/// A titled panel.
///
/// The instrument's front panel does not fold its sections away, and neither does this:
/// a control you cannot see is a control you do not know you have.
pub fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(egui::RichText::new(title).strong());
        ui.separator();
        body(ui);
    });
    ui.add_space(2.0);
}

/// A run of controls across the panel, wrapping when it runs out of width.
///
/// ⚠️ Aligned to the **top** of the row rather than egui's default centre. A cell asks
/// for its width and lets its height follow its contents, and a centred row hands each
/// one the whole remaining height to be centred in — which leaves the controls staggered
/// down the row and the rows themselves hundreds of points tall.
pub fn strip(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
    let row = egui::Layout::left_to_right(egui::Align::TOP).with_main_wrap(true);
    ui.with_layout(row, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
        body(ui);
    });
}

/// How wide a cell is, which is as wide as what stands in it.
fn width(control: Option<Control>) -> f32 {
    match control {
        Some(Control::Choice) => 156.0,
        Some(Control::Stored) | None => 140.0,
        Some(Control::Register) => 220.0,
        _ => 78.0,
    }
}

/// One field as a cell: the control, and the panel's name for it underneath.
pub fn cell(ui: &mut egui::Ui, ctx: &Ctx, field: &Field, sets: &mut Sets) {
    let kind = ctx.control.get(&field.path).copied();
    named_cell(ui, &field.path, width(kind), |ui| {
        if let Some(value) = control(ui, ctx, field) {
            sets.push((field.path.clone(), value));
        }
    });
}

/// A cell whose control is the caller's — for the two that are not one field each.
///
/// `path` names the caption; an unmapped one still gets the prettified fallback, so a
/// field the strings table has not caught up with reads as a rough name rather than as a
/// nameless knob.
pub fn named_cell(
    ui: &mut egui::Ui,
    path: &str,
    width: f32,
    body: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    ui.allocate_ui(egui::vec2(width, 0.0), |ui| {
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            body(ui);
            caption(ui, path);
        });
    })
    .response
}

/// The name under a control, in the panel's own words.
fn caption(ui: &mut egui::Ui, path: &str) {
    let rough = !strings::known(path);
    let mut text = egui::RichText::new(strings::label(path)).small();
    if rough {
        text = text.italics();
    }
    let response = ui.add(egui::Label::new(text.color(ui.visuals().weak_text_color())));
    if rough {
        response.on_hover_text(format!("{path} — this app has no name for it yet"));
    }
}

/// The control alone, without its name.
pub fn control(ui: &mut egui::Ui, ctx: &Ctx, field: &Field) -> Option<String> {
    match ctx.control.get(&field.path).copied() {
        Some(Control::Toggle) => toggle(ui, field),
        Some(Control::Choice) => choice(ui, ctx, field),
        Some(Control::Number { min, max }) => number(ui, field, min, max),
        Some(Control::Register) => register(ui, field, true),
        // A field too wide to name its values has only its stored bits, and those are an
        // engineer's business — the Advanced dump is where they are legible.
        Some(Control::Stored) | None => {
            ui.label(
                egui::RichText::new(&field.display)
                    .monospace()
                    .small()
                    .weak(),
            )
            .on_hover_text("stored as-is; see Advanced");
            None
        }
    }
}

/// A lamp with no word beside it: the cell's own caption names it.
fn toggle(ui: &mut egui::Ui, field: &Field) -> Option<String> {
    let on = field.value == "true";
    led::ui(ui, on, "").map(|want| want.to_string())
}

/// A named-value picker. Shows the panel's word for each value and sets the library's.
fn choice(ui: &mut egui::Ui, ctx: &Ctx, field: &Field) -> Option<String> {
    let offered = visibility::choices(&field.path, ctx.legal_of(&field.path), &field.value);
    let mut picked = None;
    egui::ComboBox::from_id_salt(&field.path)
        .selected_text(
            egui::RichText::new(strings::value_label(&field.path, &field.value))
                .text_style(egui::TextStyle::Small),
        )
        .width(
            ui.available_width()
                .min(width(Some(Control::Choice)) - 12.0),
        )
        .show_ui(ui, |ui| {
            for value in &offered {
                let label = strings::value_label(&field.path, value);
                if ui.selectable_label(*value == field.value, label).clicked() {
                    picked = Some(value.clone());
                }
            }
        });
    picked.filter(|value| *value != field.value)
}

/// A knob, because that is what the panel puts a continuous value on.
fn number(ui: &mut egui::Ui, field: &Field, min: i64, max: i64) -> Option<String> {
    let value: i64 = field.value.trim_start_matches('+').parse().ok()?;
    knob::ui(ui, &field.path, value, min, max).map(|moved| moved.to_string())
}

/// Nine drawbars and the positions under them. No hex: the digits are the readout.
pub fn register(ui: &mut egui::Ui, field: &Field, live: bool) -> Option<String> {
    let bits = drawbar_widget::parse(&field.value)?;
    bars(ui, drawbar_widget::bars(bits), live, drawbar_widget::BARS)
        .map(|moved| drawbar_widget::spell(drawbar_widget::bits(moved)))
}

/// The drawbars themselves, plus the digits. Returns the positions when one is pulled.
pub fn bars(
    ui: &mut egui::Ui,
    positions: [u8; drawbar_widget::BARS],
    live: bool,
    count: usize,
) -> Option<[u8; drawbar_widget::BARS]> {
    let mut moved = None;
    ui.vertical(|ui| {
        moved = drawbar_widget::ui_count(ui, positions, live, count);
        let shown = moved.unwrap_or(positions);
        ui.label(
            egui::RichText::new(drawbar_widget::digits(&shown[..count.min(shown.len())]))
                .monospace()
                .small()
                .weak(),
        );
    });
    moved
}

/// A lamp with its own word beside it, for the switches that sit inside a preset.
pub fn switch(ui: &mut egui::Ui, field: Option<&Field>, word: &str, sets: &mut Sets) {
    let Some(field) = field else {
        return;
    };
    let on = field.value == "true";
    if let Some(want) = led::ui(ui, on, word) {
        sets.push((field.path.clone(), want.to_string()));
    }
}
