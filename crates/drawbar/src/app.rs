//! The app shell: theme, the four regions, and the routing between them.
//!
//! Left is the workspace (local entities), right is the instrument, the centre
//! inspects whatever is selected in either, and the bottom keeps the activity log.

use eframe::egui;

use crate::device::Device;
use crate::editor::Editor;
use crate::inspect::Inspector;
use crate::log::Log;
use crate::sample_edit::SampleEditor;
use crate::workspace::{Origin, Workspace};

/// The three status accents. Dark panel, small signals — an instrument tool.
pub const GOOD: egui::Color32 = egui::Color32::from_rgb(0x60, 0xc0, 0x70);
pub const WARN: egui::Color32 = egui::Color32::from_rgb(0xe0, 0xa0, 0x30);
pub const BAD: egui::Color32 = egui::Color32::from_rgb(0xe0, 0x50, 0x40);

/// What the centre is showing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Centre {
    Inspect,
    Edit,
}

pub struct DrawbarApp {
    workspace: Workspace,
    device: Device,
    inspector: Inspector,
    editor: Editor,
    sample_editor: SampleEditor,
    centre: Centre,
    log: Log,
}

impl DrawbarApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> DrawbarApp {
        cc.egui_ctx.set_visuals(visuals());
        let mut log = Log::default();
        log.info("drawbar — read-only until you confirm an operation that is not");
        DrawbarApp {
            workspace: Workspace::new(cc.egui_ctx.clone()),
            device: Device::new(cc.egui_ctx.clone()),
            inspector: Inspector::default(),
            editor: Editor::default(),
            sample_editor: SampleEditor::default(),
            centre: Centre::Inspect,
            log,
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
                        None
                    }
                },
                (None, None) => {
                    self.log.error(format!("{name}: dropped with no content"));
                    None
                }
            };
            if let Some(bytes) = bytes {
                self.workspace
                    .ingest(name.clone(), Origin::File(name), bytes, &mut self.log);
            }
        }
    }
}

impl eframe::App for DrawbarApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.log.tick(ctx);
        self.workspace.poll(&mut self.log);
        self.device.poll(&mut self.log, &mut self.workspace);
        self.take_dropped_files(ctx);
        drop_hint(ctx);

        egui::TopBottomPanel::top("title").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("drawbar");
                ui.label(
                    egui::RichText::new("Nord files, and the instrument that holds them")
                        .weak()
                        .italics(),
                );
            });
        });

        egui::TopBottomPanel::bottom("activity")
            .resizable(true)
            .default_height(160.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let label = if self.log.open {
                        "▼ Activity"
                    } else {
                        "▶ Activity"
                    };
                    if ui.selectable_label(false, label).clicked() {
                        self.log.open = !self.log.open;
                    }
                    ui.label(egui::RichText::new(format!("{} entries", self.log.len())).weak());
                    if ui.small_button("Clear").clicked() {
                        self.log.clear();
                    }
                    if !self.log.open {
                        if let Some(last) = self.log.last() {
                            ui.label(egui::RichText::new(&last.text).monospace().weak());
                        }
                    }
                });
                if self.log.open {
                    ui.separator();
                    self.log.ui(ui);
                }
            });

        egui::SidePanel::left("workspace")
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| {
                ui.heading("Workspace");
                // A fresh default exists to be edited, so it opens there — the CLI's
                // target-less `edit -o`.
                if let Some(id) = self.workspace.ui(ui, &mut self.log) {
                    self.editor.open(id);
                    self.centre = Centre::Edit;
                }
            });

        egui::SidePanel::right("instrument")
            .resizable(true)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.heading("Instrument");
                self.device.ui(ui, &self.workspace, &mut self.log);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let editable = self
                .workspace
                .selected()
                .and_then(|entity| entity.entity.as_ref())
                .map(|entity| {
                    (
                        crate::editor::can_edit(entity),
                        crate::sample_edit::can_edit(entity),
                    )
                });
            let (fields, sample) = editable.unwrap_or((false, false));
            if !fields && !sample {
                self.centre = Centre::Inspect;
            }

            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.centre == Centre::Inspect, "Inspect")
                    .clicked()
                {
                    self.centre = Centre::Inspect;
                }
                ui.add_enabled_ui(fields || sample, |ui| {
                    if ui
                        .selectable_label(self.centre == Centre::Edit, "Edit")
                        .on_disabled_hover_text("this format has no editor")
                        .clicked()
                    {
                        self.centre = Centre::Edit;
                    }
                });
            });
            ui.separator();

            match self.centre {
                Centre::Inspect => {
                    if self.inspector.ui(ui, self.workspace.selected()) {
                        self.centre = Centre::Edit;
                    }
                }
                Centre::Edit if sample => {
                    self.sample_editor
                        .ui(ui, &mut self.workspace, &mut self.log)
                }
                Centre::Edit => self.editor.ui(ui, &mut self.workspace, &mut self.log),
            }
        });
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
