//! The application shell: load a file, look at it, talk to a device. All of it is
//! shared between the desktop and browser builds.

use std::io::Cursor;

use egui::{Context, RichText, Ui};
use nord_format::electro5::program::OrganModel;
use nord_format::{Entity, Program};

use crate::panel::OrganView;
use crate::{demo, device::DevicePanel, panel, theme};

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Panel,
    Device,
    About,
}

struct Loaded {
    name: String,
    size: usize,
    /// Whether re-encoding the parsed value reproduced the input byte-for-byte —
    /// `nord-format`'s central invariant, checked live on whatever you dropped in.
    round_trip: Result<(), String>,
    entity: Entity,
    view: OrganView,
}

pub struct NordGui {
    tab: Tab,
    loaded: Option<Loaded>,
    error: Option<String>,
    device: DevicePanel,
}

impl NordGui {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let mut app = Self {
            tab: Tab::Panel,
            loaded: None,
            error: None,
            device: DevicePanel::default(),
        };
        app.load(demo::NAME.to_string(), demo::bytes());
        app
    }

    /// Parse bytes into an [`Entity`], then immediately re-encode them to check the
    /// round-trip. Everything the UI shows hangs off this.
    fn load(&mut self, name: String, bytes: Vec<u8>) {
        self.error = None;
        let mut entity = match nord_format::from_stream(&mut Cursor::new(&bytes)) {
            Ok(entity) => entity,
            Err(e) => {
                self.error = Some(format!("{name}: {e}"));
                self.loaded = None;
                return;
            }
        };

        let round_trip = match nord_format::to_bytes(&mut entity) {
            Ok(out) if out == bytes => Ok(()),
            Ok(out) => Err(match out.iter().zip(&bytes).position(|(a, b)| a != b) {
                Some(at) => format!("differs at {at:#x}"),
                None => format!("length differs: {} in, {} out", bytes.len(), out.len()),
            }),
            Err(e) => Err(e.to_string()),
        };

        let view = match &entity {
            Entity::Program(Program::Electro5(p)) => OrganView::of(p),
            _ => OrganView {
                model: OrganModel::B3,
                preset: 1,
            },
        };

        self.loaded = Some(Loaded {
            name,
            size: bytes.len(),
            round_trip,
            entity,
            view,
        });
        self.tab = Tab::Panel;
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("nord")
                    .size(20.0)
                    .color(theme::RED_TEXT)
                    .strong(),
            );
            ui.label(RichText::new("gui").size(20.0).color(theme::DIM));
            ui.add_space(12.0);
            ui.selectable_value(&mut self.tab, Tab::Panel, "panel");
            ui.selectable_value(&mut self.tab, Tab::Device, "device");
            ui.selectable_value(&mut self.tab, Tab::About, "about");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                #[cfg(not(target_arch = "wasm32"))]
                if ui.button("open…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Nord files", &["ne5p", "ne5t", "ne5s", "npno", "nsmp"])
                        .pick_file()
                    {
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.display().to_string());
                        match std::fs::read(&path) {
                            Ok(bytes) => self.load(name, bytes),
                            Err(e) => self.error = Some(format!("{name}: {e}")),
                        }
                    }
                }
                if ui.button("demo program").clicked() {
                    self.load(demo::NAME.to_string(), demo::bytes());
                }
                if let Some(loaded) = &self.loaded {
                    match &loaded.round_trip {
                        Ok(()) => {
                            ui.label(RichText::new("round-trip ✔").color(theme::AMBER).size(12.0))
                        }
                        Err(why) => ui.label(
                            RichText::new(format!("round-trip ✘ {why}"))
                                .color(theme::RED_TEXT)
                                .size(12.0),
                        ),
                    };
                    ui.label(
                        RichText::new(format!("{} · {} bytes", loaded.name, loaded.size))
                            .color(theme::DIM)
                            .size(12.0),
                    );
                }
            });
        });
    }

    fn about(&self, ui: &mut Ui) {
        theme::card_ui(ui, |ui| {
            ui.label(
                RichText::new("what this is")
                    .color(theme::RED_TEXT)
                    .strong(),
            );
            ui.add_space(4.0);
            ui.label(
                "A proof of concept: one egui application over nord-format and nord-usb, \
                 built as a desktop binary and as wasm for the browser. The panel, the \
                 device tab and every widget are the same code on both.",
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new("what differs by platform")
                    .color(theme::RED_TEXT)
                    .strong(),
            );
            ui.add_space(4.0);
            ui.label(
                "Two things. How a future is driven — pollster natively, a single poll \
                 cycle on wasm. And which transports exist: USB is desktop-only, because \
                 a browser would need WebUSB and nord-usb has no web transport yet. The \
                 recorded capture runs on both, unchanged.",
            );
            ui.add_space(8.0);
            ui.label(RichText::new("read-only").color(theme::RED_TEXT).strong());
            ui.add_space(4.0);
            ui.label(
                "Nothing here writes to an instrument. nord-usb can delete, rename and \
                 overwrite slots; none of that is reachable from this UI, and files are \
                 only ever parsed and re-encoded in memory.",
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "Drop a .ne5p / .ne5t / .ne5s / .npno / .nsmp file anywhere on the \
                     window to load it.",
                )
                .color(theme::DIM),
            );
        });
    }

    /// Files dropped on the window. The browser hands over the bytes directly; natively
    /// there is only a path, so read it here.
    fn take_dropped(&mut self, ctx: &Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        let Some(file) = dropped.into_iter().next() else {
            return;
        };

        let name = if file.name.is_empty() {
            "dropped file".to_string()
        } else {
            file.name.clone()
        };

        if let Some(bytes) = &file.bytes {
            self.load(name, bytes.to_vec());
            return;
        }

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = &file.path {
            match std::fs::read(path) {
                Ok(bytes) => self.load(name, bytes),
                Err(e) => self.error = Some(format!("{name}: {e}")),
            }
        }
    }
}

impl eframe::App for NordGui {
    /// egui 0.35 hands the app a root [`Ui`] covering the viewport rather than a
    /// [`Context`]; the panels are shown inside it.
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let viewport = ui.max_rect();

        self.take_dropped(&ctx);

        // A program read off an instrument goes straight into the panel tab.
        if let Some((name, bytes)) = self.device.fetched.take() {
            self.load(name, bytes);
        }

        egui::Panel::top(egui::Id::new("toolbar"))
            .frame(
                egui::Frame::NONE
                    .fill(theme::CARD)
                    .inner_margin(egui::Margin::symmetric(12, 8)),
            )
            .show(ui, |ui| self.toolbar(ui));

        egui::CentralPanel::no_frame()
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::same(12)))
            .show(ui, |ui| {
                if let Some(error) = &self.error {
                    ui.label(RichText::new(error).color(theme::RED_TEXT));
                    ui.add_space(8.0);
                }
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.tab {
                        Tab::Panel => match &mut self.loaded {
                            Some(loaded) => panel::entity(ui, &loaded.entity, &mut loaded.view),
                            None => {
                                ui.label(RichText::new("nothing loaded").color(theme::DIM));
                            }
                        },
                        Tab::Device => self.device.ui(ui),
                        Tab::About => self.about(ui),
                    });
            });

        // A drop target hint while a file is over the window.
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("drop_hint"),
            ));
            painter.rect_filled(viewport, 0, egui::Color32::from_black_alpha(180));
            painter.text(
                viewport.center(),
                egui::Align2::CENTER_CENTER,
                "drop to parse",
                egui::FontId::proportional(28.0),
                theme::RED_TEXT,
            );
        }
    }
}
