//! The activity log: a bounded record of what the app did, oldest dropped first.

use std::collections::VecDeque;

use eframe::egui;

/// Entries kept before the oldest is dropped. A session that opens a hundred files
/// still fits; a runaway loop cannot grow the app without bound.
const CAPACITY: usize = 500;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn color(self, visuals: &egui::Visuals) -> egui::Color32 {
        match self {
            Level::Info => visuals.weak_text_color(),
            Level::Warn => egui::Color32::from_rgb(0xe0, 0xa0, 0x30),
            Level::Error => egui::Color32::from_rgb(0xe0, 0x50, 0x40),
        }
    }
}

pub struct Entry {
    pub level: Level,
    pub text: String,
    /// Seconds since the app started.
    pub at: f64,
}

pub struct Log {
    entries: VecDeque<Entry>,
    /// egui's frame time, refreshed by [`Log::tick`].
    ///
    /// ⚠️ `std::time::Instant::now()` traps on `wasm32-unknown-unknown`, so the log
    /// cannot read a clock of its own; the timeline is elapsed seconds, not wall time.
    clock: f64,
    pub open: bool,
}

impl Default for Log {
    fn default() -> Log {
        Log {
            entries: VecDeque::new(),
            clock: 0.0,
            open: true,
        }
    }
}

impl Log {
    /// Take this frame's time. Call once per frame, before anything that logs.
    pub fn tick(&mut self, ctx: &egui::Context) {
        self.clock = ctx.input(|i| i.time);
    }

    pub fn info(&mut self, text: impl Into<String>) {
        self.push(Level::Info, text);
    }

    pub fn warn(&mut self, text: impl Into<String>) {
        self.push(Level::Warn, text);
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.push(Level::Error, text);
    }

    fn push(&mut self, level: Level, text: impl Into<String>) {
        if self.entries.len() == CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(Entry {
            level,
            text: text.into(),
            at: self.clock,
        });
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// The newest entry, for the collapsed header's one-line summary.
    pub fn last(&self) -> Option<&Entry> {
        self.entries.back()
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for entry in &self.entries {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        ui.label(
                            egui::RichText::new(format!("{:>8.1}s", entry.at))
                                .monospace()
                                .weak(),
                        );
                        ui.label(
                            egui::RichText::new(&entry.text)
                                .monospace()
                                .color(entry.level.color(ui.visuals())),
                        );
                    });
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_oldest_entry_is_dropped_once_the_ring_is_full() {
        let mut log = Log::default();
        for n in 0..CAPACITY + 10 {
            log.info(format!("line {n}"));
        }
        assert_eq!(log.len(), CAPACITY);
        assert_eq!(log.iter().next().unwrap().text, "line 10");
        assert_eq!(log.last().unwrap().text, format!("line {}", CAPACITY + 9));
    }
}
