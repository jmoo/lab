//! A dark red-on-charcoal palette, in the spirit of the instrument's own panel.

use egui::{Color32, Context, CornerRadius, Stroke};

/// The Nord red, off the front panel.
pub const RED: Color32 = Color32::from_rgb(0xc0, 0x2a, 0x2a);
/// A brighter red for text that has to stay legible on charcoal.
pub const RED_TEXT: Color32 = Color32::from_rgb(0xe8, 0x5c, 0x5c);
/// Lit segment / engaged indicator.
pub const AMBER: Color32 = Color32::from_rgb(0xf2, 0xa5, 0x3a);
pub const INK: Color32 = Color32::from_rgb(0x14, 0x15, 0x18);
pub const CARD: Color32 = Color32::from_rgb(0x1e, 0x20, 0x25);
pub const DIM: Color32 = Color32::from_rgb(0x8a, 0x8f, 0x99);

pub fn apply(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = INK;
    visuals.window_fill = CARD;
    visuals.extreme_bg_color = Color32::from_rgb(0x0d, 0x0e, 0x10);
    visuals.faint_bg_color = CARD;
    visuals.widgets.noninteractive.bg_fill = CARD;
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(0x2a, 0x2d, 0x33);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x3a, 0x3e, 0x46);
    visuals.widgets.active.bg_fill = RED;
    visuals.selection.bg_fill = RED.gamma_multiply(0.8);
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(4);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(4);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(4);
    visuals.widgets.active.corner_radius = CornerRadius::same(4);
    // Both themes, so the app looks the same whatever the system asks for.
    ctx.all_styles_mut(|style| {
        style.visuals = visuals.clone();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
    });
}

/// A card filling the width of whatever it is placed in — the page, or one column.
pub fn card_ui<R>(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui) -> R) -> R {
    card()
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui)
        })
        .inner
}

/// The card every section of the panel sits in.
pub fn card() -> egui::Frame {
    egui::Frame::NONE
        .fill(CARD)
        .inner_margin(egui::Margin::same(12))
        .corner_radius(CornerRadius::same(8))
        .stroke(Stroke::new(1.0, Color32::from_rgb(0x2c, 0x2f, 0x36)))
}
