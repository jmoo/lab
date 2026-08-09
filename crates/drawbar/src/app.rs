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

/// The three status accents. Dark panel, small signals — an instrument tool.
pub const GOOD: egui::Color32 = egui::Color32::from_rgb(0x60, 0xc0, 0x70);
pub const WARN: egui::Color32 = egui::Color32::from_rgb(0xe0, 0xa0, 0x30);
pub const BAD: egui::Color32 = egui::Color32::from_rgb(0xe0, 0x50, 0x40);

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
}

impl DrawbarApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> DrawbarApp {
        cc.egui_ctx.set_visuals(visuals());
        DrawbarApp {
            workspace: Workspace::new(cc.egui_ctx.clone()),
            device: Device::new(cc.egui_ctx.clone()),
            browser: Browser::default(),
            tabs: Tabs::default(),
            document: Document::default(),
            log: Log::default(),
        }
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                ui.label(
                    egui::RichText::new("your sounds, here and on the instrument")
                        .weak()
                        .italics(),
                );
            });
        });

        self.status_strip(ctx);

        let mut acts = Vec::new();
        egui::SidePanel::left("places")
            .resizable(true)
            .default_width(300.0)
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
fn visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(0x16, 0x17, 0x19);
    visuals.window_fill = egui::Color32::from_rgb(0x1c, 0x1d, 0x20);
    visuals.faint_bg_color = egui::Color32::from_rgb(0x22, 0x23, 0x26);
    visuals.selection.bg_fill = egui::Color32::from_rgb(0x7a, 0x24, 0x24);
    visuals.hyperlink_color = BAD;
    visuals
}
