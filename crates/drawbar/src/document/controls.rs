//! One row per control, and the pieces every document builds rows out of.
//!
//! A row reads its value out of the field list and hands back a `path = value` set when
//! the user moves it. Nothing here writes: the document collects every set the frame
//! produced and applies them together, so a control that owns two fields moves both or
//! neither.

use std::collections::HashMap;

use eframe::egui;
use nord_format::fields::Field;

use crate::drawbar_widget;
use crate::fields::Control;
use crate::strings;
use crate::visibility;

/// The width the labels line up at, so a section reads as a column of values.
const LABEL_W: f32 = 168.0;

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

/// A label in the left column, sized so the controls line up.
pub fn label(ui: &mut egui::Ui, path: &str) {
    let text = strings::label(path);
    let rough = !strings::known(path);
    let mut rich = egui::RichText::new(text);
    if rough {
        rich = rich.italics();
    }
    let response = ui.add_sized(
        [LABEL_W, ui.spacing().interact_size.y],
        egui::Label::new(rich).halign(egui::Align::LEFT),
    );
    if rough {
        response.on_hover_text(format!("{path} — this app has no name for it yet"));
    }
}

/// One field as its own row: the label, then whatever control its type asks for.
pub fn row(ui: &mut egui::Ui, ctx: &Ctx, field: &Field, sets: &mut Sets) {
    ui.horizontal(|ui| {
        label(ui, &field.path);
        if let Some(value) = control(ui, ctx, field) {
            sets.push((field.path.clone(), value));
        }
    });
}

/// The control alone, without its label — for the rows that arrange themselves.
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

fn toggle(ui: &mut egui::Ui, field: &Field) -> Option<String> {
    let mut on = field.value == "true";
    ui.checkbox(&mut on, "").changed().then(|| on.to_string())
}

/// A named-value picker. Shows the panel's word for each value and sets the library's.
fn choice(ui: &mut egui::Ui, ctx: &Ctx, field: &Field) -> Option<String> {
    let offered = visibility::choices(&field.path, ctx.legal_of(&field.path), &field.value);
    let mut picked = None;
    egui::ComboBox::from_id_salt(&field.path)
        .selected_text(strings::value_label(&field.path, &field.value))
        .width(190.0)
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

fn number(ui: &mut egui::Ui, field: &Field, min: i64, max: i64) -> Option<String> {
    let mut value: i64 = field.value.trim_start_matches('+').parse().ok()?;
    let before = value;
    ui.add(egui::DragValue::new(&mut value).range(min..=max));
    (value != before).then(|| value.to_string())
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

/// A checkbox with its own word beside it, for the switches that sit inside a row.
pub fn switch(ui: &mut egui::Ui, field: Option<&Field>, word: &str, sets: &mut Sets) {
    let Some(field) = field else {
        return;
    };
    let mut on = field.value == "true";
    if ui.checkbox(&mut on, word).changed() {
        sets.push((field.path.clone(), on.to_string()));
    }
}
