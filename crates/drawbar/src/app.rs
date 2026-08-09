//! The app shell: theme, the three regions, and the routing between them.
//!
//! The sidebar is the browser — this computer and the instrument. The centre is a tab
//! per open document. The bottom is one line of plain words, which opens into the full
//! activity log when there is a reason to read it.

use eframe::egui;

use crate::browser::{self, Browser};
use crate::device::Device;
use crate::document::Document;
use crate::log::Log;
use crate::tabs::Tabs;
use crate::workspace::{Origin, Workspace};

/// The three status accents, and the panel's own red.
///
/// ⚠️ Each theme gets its own set. A signal picked to glow on a black panel is washed out
/// on paper, and one picked for paper disappears on the panel — so nothing here is a
/// constant, and nothing outside this module spells a status colour for itself.
pub fn good(visuals: &egui::Visuals) -> egui::Color32 {
    match visuals.dark_mode {
        true => egui::Color32::from_rgb(0x60, 0xc0, 0x70),
        false => egui::Color32::from_rgb(0x1e, 0x7a, 0x36),
    }
}

pub fn warn(visuals: &egui::Visuals) -> egui::Color32 {
    match visuals.dark_mode {
        true => egui::Color32::from_rgb(0xe0, 0xa0, 0x30),
        false => egui::Color32::from_rgb(0x8a, 0x5a, 0x00),
    }
}

pub fn bad(visuals: &egui::Visuals) -> egui::Color32 {
    match visuals.dark_mode {
        true => egui::Color32::from_rgb(0xe0, 0x50, 0x40),
        false => egui::Color32::from_rgb(0xb3, 0x2a, 0x1e),
    }
}

/// The red the instrument itself is: a lit lamp, a knob's travelled arc, a selected row.
pub fn accent(visuals: &egui::Visuals) -> egui::Color32 {
    match visuals.dark_mode {
        true => egui::Color32::from_rgb(0xd6, 0x46, 0x3a),
        false => egui::Color32::from_rgb(0xa8, 0x33, 0x2a),
    }
}

/// Which theme the operator asked for.
///
/// `System` is the unset state: the app follows the desktop's or the browser's own
/// preference until it is told otherwise, and goes back to following it when the choice
/// is cycled past.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeChoice {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    /// Where the choice is kept between sessions.
    const KEY: &'static str = "drawbar.theme";

    fn read(text: &str) -> ThemeChoice {
        match text {
            "light" => ThemeChoice::Light,
            "dark" => ThemeChoice::Dark,
            _ => ThemeChoice::System,
        }
    }

    fn stored(self) -> &'static str {
        match self {
            ThemeChoice::System => "system",
            ThemeChoice::Light => "light",
            ThemeChoice::Dark => "dark",
        }
    }

    fn next(self) -> ThemeChoice {
        match self {
            ThemeChoice::System => ThemeChoice::Light,
            ThemeChoice::Light => ThemeChoice::Dark,
            ThemeChoice::Dark => ThemeChoice::System,
        }
    }

    /// ⚠️ A word, not a sun or a moon: the bundled fonts have no glyph for either, and a
    /// missing one renders as an empty box.
    fn label(self) -> &'static str {
        match self {
            ThemeChoice::System => "Theme: auto",
            ThemeChoice::Light => "Theme: light",
            ThemeChoice::Dark => "Theme: dark",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            ThemeChoice::System => "following the system — click for light",
            ThemeChoice::Light => "held light — click for dark",
            ThemeChoice::Dark => "held dark — click to follow the system again",
        }
    }

    fn preference(self) -> egui::ThemePreference {
        match self {
            ThemeChoice::System => egui::ThemePreference::System,
            ThemeChoice::Light => egui::ThemePreference::Light,
            ThemeChoice::Dark => egui::ThemePreference::Dark,
        }
    }
}

/// A small filled dot: something changed here, or something is attached here.
///
/// ⚠️ Painted rather than typed. The bundled fonts have no glyph for `●`, and a missing
/// one renders as an empty box — which reads as a checkbox nobody can tick.
pub fn dot(ui: &mut egui::Ui, color: egui::Color32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(9.0, 9.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.5, color);
    response
}

pub struct DrawbarApp {
    workspace: Workspace,
    device: Device,
    browser: Browser,
    tabs: Tabs,
    document: Document,
    log: Log,
    theme: ThemeChoice,
    /// The list's revision as the store last saw it.
    saved: u64,
    /// When the store was last caught up, on egui's own clock.
    saved_at: f64,
}

impl DrawbarApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> DrawbarApp {
        // Both faces are dressed up front, so the system flipping from light to dark mid
        // session lands on this app's own colours rather than egui's defaults.
        cc.egui_ctx.set_visuals_of(egui::Theme::Dark, dark());
        cc.egui_ctx.set_visuals_of(egui::Theme::Light, light());
        let theme = cc
            .storage
            .and_then(|storage| storage.get_string(ThemeChoice::KEY))
            .map_or(ThemeChoice::default(), |text| ThemeChoice::read(&text));
        cc.egui_ctx.set_theme(theme.preference());
        let mut app = DrawbarApp {
            workspace: Workspace::new(cc.egui_ctx.clone()),
            device: Device::new(cc.egui_ctx.clone()),
            browser: Browser::default(),
            tabs: Tabs::default(),
            document: Document::default(),
            log: Log::default(),
            theme,
            saved: 0,
            saved_at: 0.0,
        };
        if let Some(storage) = cc.storage {
            crate::store::load(storage, &mut app.workspace, &mut app.log);
        }
        app.saved = app.workspace.revision();
        app
    }

    /// Ingest anything dropped on the window.
    ///
    /// The web backend fills `bytes` and the native backend fills `path`, so both are
    /// handled rather than cfg'd apart.
    fn take_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            let name = match (file.name.is_empty(), &file.path) {
                (false, _) => file.name.clone(),
                (true, Some(path)) => path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string()),
                (true, None) => "dropped".to_string(),
            };
            let bytes = match (&file.bytes, &file.path) {
                (Some(bytes), _) => Some(bytes.to_vec()),
                (None, Some(path)) => match std::fs::read(path) {
                    Ok(bytes) => Some(bytes),
                    Err(e) => {
                        self.log.error(format!("{name}: {e}"));
                        self.log.trouble(format!("Could not read {name}."));
                        None
                    }
                },
                (None, None) => {
                    self.log
                        .trouble(format!("{name} arrived with no contents."));
                    None
                }
            };
            if let Some(bytes) = bytes {
                self.workspace
                    .ingest(name.clone(), Origin::File(name), bytes, &mut self.log);
            }
        }
    }

    /// Catch the store up with the list.
    ///
    /// ⚠️ eframe's own periodic save runs at the end of a frame, and egui only paints
    /// when something asks it to — a change made and then left alone can sit unwritten
    /// for as long as the window goes untouched. So the write happens here, from the
    /// frame that made the change, and eframe's `save` is left as the way out.
    ///
    /// Rate-limited because a drag changes the list on every frame it moves, and the
    /// whole list is re-encoded each time.
    fn keep_up(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        /// How often the store is allowed to be rewritten, in seconds.
        const EVERY: f64 = 2.0;

        if self.workspace.revision() == self.saved {
            return;
        }
        let now = ctx.input(|i| i.time);
        if now - self.saved_at < EVERY {
            // Nothing else may be about to ask for a frame, and the write is still owed.
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
            return;
        }
        let Some(storage) = frame.storage_mut() else {
            return;
        };
        crate::store::save(storage, &self.workspace, &mut self.log);
        self.saved = self.workspace.revision();
        self.saved_at = now;
    }

    /// One line of plain words, and the whole log behind it.
    fn status_strip(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .resizable(self.log.open)
            .default_height(if self.log.open { 200.0 } else { 28.0 })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let line = match &self.device.state.in_flight {
                        Some(words) => {
                            ui.spinner();
                            egui::RichText::new(&words.doing)
                        }
                        None => {
                            let (level, text) = self.log.status();
                            egui::RichText::new(text).color(level.color(ui.visuals()))
                        }
                    };
                    if ui
                        .add(egui::Label::new(line).sense(egui::Sense::click()))
                        .clicked()
                    {
                        self.log.open = !self.log.open;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = match self.log.open {
                            true => "Hide details",
                            false => "Details",
                        };
                        if ui.small_button(label).clicked() {
                            self.log.open = !self.log.open;
                        }
                        if self.log.open && ui.small_button("Clear").clicked() {
                            self.log.clear();
                        }
                    });
                });
                if self.log.open {
                    ui.separator();
                    self.log.ui(ui);
                }
            });
    }
}

impl eframe::App for DrawbarApp {
    /// How long a change may sit unwritten.
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(5)
    }

    /// eframe calls this on its own timer and on the way out, so an edit is kept
    /// without anyone asking for it to be.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        crate::store::save(storage, &self.workspace, &mut self.log);
        storage.set_string(ThemeChoice::KEY, self.theme.stored().to_string());
        self.saved = self.workspace.revision();
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.log.tick(ctx);
        self.workspace.poll(&mut self.log);
        self.device
            .poll(&mut self.log, &mut self.workspace, &mut self.tabs);
        self.tabs.prune(&self.workspace);
        self.take_dropped_files(ctx);
        drop_hint(ctx);

        egui::TopBottomPanel::top("title").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("drawbar");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button(self.theme.label())
                        .on_hover_text(self.theme.hint())
                        .clicked()
                    {
                        self.theme = self.theme.next();
                        ctx.set_theme(self.theme.preference());
                        // Written from the frame that changed it: eframe's own save runs
                        // on a timer, and a choice made and then left alone would sit
                        // unwritten for as long as the window goes untouched.
                        if let Some(storage) = frame.storage_mut() {
                            storage.set_string(ThemeChoice::KEY, self.theme.stored().to_string());
                        }
                    }
                });
            });
        });

        self.status_strip(ctx);

        let mut acts = Vec::new();
        egui::SidePanel::left("places")
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                acts = self.browser.ui(ui, &self.workspace, &self.device);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.tabs.ui(ui, &self.workspace);
            ui.separator();
            let Some(id) = self.tabs.active() else {
                ui.label(
                    egui::RichText::new("Double-click something in the sidebar to open it.")
                        .weak()
                        .italics(),
                );
                return;
            };
            // ⚠️ Never a file export. Cmd+S means "keep what I did", which for something
            // read off the instrument is a promise to send it back.
            if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
                self.document.stage(id, &mut self.workspace, &mut self.log);
            }
            let sent = self.document.ui(
                ui,
                id,
                self.tabs.opened(id),
                &mut self.workspace,
                &mut self.device,
                &mut self.log,
            );
            if let Some(send) = sent {
                acts.push(browser::Act::Send {
                    id: send.id,
                    class: send.class,
                    at: send.at,
                });
            }
        });

        browser::apply(
            &mut self.browser,
            acts,
            &mut self.workspace,
            &mut self.device,
            &mut self.tabs,
            &mut self.log,
        );
        // Last, so a command the user just asked for is ahead of the background read of
        // the tree in the one slot the protocol allows.
        self.device.pump();

        self.keep_up(ctx, frame);
    }
}

/// Dim the window while files hover, so a drop has somewhere it visibly lands.
fn drop_hint(ctx: &egui::Context) {
    if ctx.input(|i| i.raw.hovered_files.is_empty()) {
        return;
    }
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("drop_hint"),
    ));
    let screen = ctx.screen_rect();
    painter.rect_filled(screen, 0.0, egui::Color32::from_black_alpha(180));
    painter.text(
        screen.center(),
        egui::Align2::CENTER_CENTER,
        "Drop to open",
        egui::FontId::proportional(28.0),
        egui::Color32::WHITE,
    );
}

/// Dark, panel-like, with the accents kept for status alone.
fn dark() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(0x16, 0x17, 0x19);
    visuals.window_fill = egui::Color32::from_rgb(0x1c, 0x1d, 0x20);
    visuals.faint_bg_color = egui::Color32::from_rgb(0x22, 0x23, 0x26);
    visuals.selection.bg_fill = egui::Color32::from_rgb(0x7a, 0x24, 0x24);
    visuals.hyperlink_color = bad(&visuals);
    visuals
}

/// The same instrument under work light: paper rather than panel, and the reds pulled
/// down to where they still read against it.
fn light() -> egui::Visuals {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = egui::Color32::from_rgb(0xf2, 0xf1, 0xee);
    visuals.window_fill = egui::Color32::from_rgb(0xfa, 0xf9, 0xf7);
    visuals.faint_bg_color = egui::Color32::from_rgb(0xe6, 0xe4, 0xdf);
    visuals.selection.bg_fill = egui::Color32::from_rgb(0xe4, 0xbc, 0xb8);
    visuals.selection.stroke.color = egui::Color32::from_rgb(0x3a, 0x14, 0x10);
    visuals.hyperlink_color = bad(&visuals);
    visuals
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The choice survives a trip through the store, and cycling reaches every state and
    /// comes back to following the system.
    #[test]
    fn the_theme_choice_round_trips_and_cycles_home() {
        let mut choice = ThemeChoice::System;
        let mut seen = Vec::new();
        for _ in 0..3 {
            seen.push(choice.stored());
            assert_eq!(ThemeChoice::read(choice.stored()), choice);
            choice = choice.next();
        }
        assert_eq!(seen, ["system", "light", "dark"]);
        assert_eq!(choice, ThemeChoice::System, "the cycle closes");
        // Anything the store cannot account for is the unset state, never a forced one.
        assert_eq!(ThemeChoice::read("moonlight"), ThemeChoice::System);
        assert_eq!(ThemeChoice::read(""), ThemeChoice::System);
    }

    /// Every accent has to be legible on the background it is painted over, and the two
    /// sets must not be the same colour twice.
    #[test]
    fn each_theme_has_its_own_accents() {
        let (dark, light) = (dark(), light());
        assert!(dark.dark_mode && !light.dark_mode);
        for accent in [good, warn, bad, accent] {
            assert_ne!(accent(&dark), accent(&light));
        }
        // The panel is dark and the paper is light, whatever egui's own defaults do.
        assert!(dark.panel_fill.intensity() < 0.2);
        assert!(light.panel_fill.intensity() > 0.8);
    }
}
