//! Rendering a decoded [`Entity`] as something that looks like the instrument's panel.
//!
//! Read-only throughout: every value shown comes off the parsed file, and nothing here
//! writes back. Where the format carries a value the panel does not (the stored `0..127`
//! behind a `0..10` knob, an unrecognised selector), both are shown rather than one
//! being quietly rounded away.

use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Ui, Vec2};
use nord_format::common::bank::Item;
use nord_format::electro5::program::{OrganModel, Program as Electro5Program};
use nord_format::electro5::{Instrument, Level};
use nord_format::{Entity, Program, Settings, Song};

use crate::theme;

/// Which organ registration the user is looking at. The file keeps all four models and
/// both presets, so the panel can browse them; the program's own selection is marked.
pub struct OrganView {
    pub model: OrganModel,
    pub preset: u8,
}

impl OrganView {
    /// Follow whatever the program has selected. b3+bass reads the B3's storage.
    pub fn of(program: &Electro5Program) -> Self {
        let model = program.organ_type().storage().unwrap_or(OrganModel::B3);
        Self {
            model,
            preset: program.organ().preset(model),
        }
    }
}

pub fn entity(ui: &mut Ui, entity: &Entity, view: &mut OrganView) {
    match entity {
        Entity::Program(Program::Electro5(p)) => program(ui, p, view),
        Entity::Song(Song::Electro5(s)) => {
            let l = s.location();
            card(ui, "set / song (ne5t)", |ui| {
                kv(
                    ui,
                    "location",
                    &format!("bank {} slot {}", l.x() + 1, l.y() + 1),
                );
                for slot in 0..4u16 {
                    let p = s.get(slot);
                    kv(
                        ui,
                        &format!("slot {}", slot + 1),
                        &format!("program bank {} slot {}", p.x() + 1, p.y() + 1),
                    );
                }
            });
        }
        Entity::Settings(Settings::Electro5(s)) => card(ui, "settings (ne5s)", |ui| {
            ui.label(
                RichText::new("field decode pending specimens — raw body:")
                    .color(theme::DIM)
                    .size(12.0),
            );
            let hex: String = s.raw().iter().map(|b| format!("{b:02x} ")).collect();
            ui.label(RichText::new(hex).monospace().size(11.0));
        }),
        Entity::Piano(_) => card(ui, "piano (npno)", |ui| {
            ui.label(RichText::new("header / reference only").color(theme::DIM));
        }),
        Entity::Sample(_) => card(ui, "sample (nsmp)", |ui| {
            ui.label(RichText::new("header / reference only").color(theme::DIM));
        }),
    }
}

fn program(ui: &mut Ui, p: &Electro5Program, view: &mut OrganView) {
    let l = p.location();

    card(ui, "program", |ui| {
        ui.horizontal_wrapped(|ui| {
            kv(
                ui,
                "location",
                &format!("bank {} slot {}", l.x() + 1, l.y() + 1),
            );
            ui.add_space(16.0);
            kv(
                ui,
                "split",
                &if p.split() {
                    format!("{:?}", p.split_point())
                } else {
                    "off".to_string()
                },
            );
            ui.add_space(16.0);
            kv(
                ui,
                "transpose",
                &format!(
                    "{:+} ({})",
                    p.transpose().inner(),
                    if p.transpose_enabled() { "on" } else { "off" }
                ),
            );
            ui.add_space(16.0);
            kv(
                ui,
                "part mix",
                &format!("{} lower/upper %", p.part_mix().as_string()),
            );
        });
        ui.add_space(6.0);
        meter(ui, "gain", p.gain());
    });

    ui.add_space(8.0);
    ui.columns(2, |cols| {
        part(
            &mut cols[0],
            "lower",
            p.lower_part(),
            p.lower_octave_shift().inner(),
            p.lower_sustain(),
            p.lower_control(),
        );
        part(
            &mut cols[1],
            "upper",
            p.upper_part(),
            p.upper_octave_shift().inner(),
            p.upper_sustain(),
            p.upper_control(),
        );
    });

    ui.add_space(8.0);
    organ(ui, p, view);

    ui.add_space(8.0);
    effects(ui, p);

    ui.add_space(8.0);
    ui.columns(2, |cols| {
        let piano = p.piano();
        card(&mut cols[0], "piano panel", |ui| {
            kv(ui, "category", &piano.category.to_string());
            kv(ui, "model", &piano.piano_model.to_string());
            kv(ui, "clav model", &piano.clav_model.to_string());
            kv(ui, "acoustics", &piano.acoustics.to_string());
            kv(ui, "touch", &piano.touch.to_string());
            kv(ui, "mono", if piano.mono { "yes" } else { "no" });
            kv(ui, "depends on", &dep_id(piano.id));
        });
        let sample = p.sample();
        card(&mut cols[1], "sample panel", |ui| {
            kv(ui, "slot", &sample.number.to_string());
            meter(ui, "attack", sample.attack);
            meter(ui, "decay/rel", sample.decay_release);
            kv(ui, "dynamics", &sample.dynamics.to_string());
            kv(ui, "filter", if sample.filter { "on" } else { "off" });
            kv(ui, "depends on", &dep_id(sample.id));
        });
    });
}

fn part(ui: &mut Ui, name: &str, instrument: Instrument, octave: i8, sustain: bool, control: bool) {
    card(ui, name, |ui| {
        ui.label(
            RichText::new(instrument.as_str())
                .size(22.0)
                .color(theme::RED_TEXT)
                .strong(),
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            chip(ui, &format!("octave {octave:+}"), octave != 0);
            chip(ui, "sustain", sustain);
            chip(ui, "control", control);
        });
    });
}

fn organ(ui: &mut Ui, p: &Electro5Program, view: &mut OrganView) {
    let selected = p.organ_type();
    let organ = p.organ();

    card(ui, "organ", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("program selects")
                    .color(theme::DIM)
                    .size(12.0),
            );
            ui.label(
                RichText::new(selected.to_string())
                    .color(theme::AMBER)
                    .strong(),
            );
            ui.add_space(12.0);
            ui.separator();
            for (model, label) in [
                (OrganModel::B3, "b3"),
                (OrganModel::Vox, "vox"),
                (OrganModel::Farfisa, "farfisa"),
                (OrganModel::Pipe, "pipe"),
            ] {
                let live = selected.storage() == Some(model);
                let text = if live {
                    RichText::new(format!("• {label}")).color(theme::AMBER)
                } else {
                    RichText::new(label.to_string())
                };
                ui.selectable_value(&mut view.model, model, text);
            }
            ui.separator();
            for preset in 1..=2u8 {
                let live = organ.preset(view.model) == preset;
                let text = if live {
                    RichText::new(format!("• preset {preset}")).color(theme::AMBER)
                } else {
                    RichText::new(format!("preset {preset}"))
                };
                ui.selectable_value(&mut view.preset, preset, text);
            }
        });

        ui.add_space(8.0);

        // In b3+bass, preset 1 *is* the bass manual: two drawbars kept outside the
        // nine-nibble block, whose contents are stale. Showing the nine would be a lie.
        let bass_manual = selected.is_b3_bass() && view.model == OrganModel::B3 && view.preset == 1;
        if bass_manual {
            let bars = organ.b3_bass_drawbars();
            drawbars(ui, [bars[0], bars[1], 0, 0, 0, 0, 0, 0, 0], 2);
            ui.label(
                RichText::new("b3+bass preset 1 is the bass manual — only bars 1–2 exist")
                    .color(theme::DIM)
                    .size(12.0),
            );
        } else {
            drawbars(ui, organ.drawbars(view.model, view.preset), 9);
        }

        if view.model == OrganModel::Farfisa {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("tabs").color(theme::DIM).size(12.0));
                for on in organ.farfisa_tabs(view.preset) {
                    chip(ui, if on { "on" } else { "off" }, on);
                }
            });
            ui.label(
                RichText::new(
                    "Farfisa's drawbars are on/off tabs on the panel, but the file still \
                     stores nine positions — both are shown.",
                )
                .color(theme::DIM)
                .size(11.0),
            );
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            match organ.vib_type(view.model) {
                Some(v) => chip(
                    ui,
                    &format!("vib {v:?}"),
                    organ.vib_on(view.model, view.preset),
                ),
                None => chip(ui, "no vibrato", false),
            }
            if view.model == OrganModel::B3 {
                let on = organ.b3_perc_on(view.preset);
                chip(ui, &format!("perc {:?}", organ.b3_perc_speed()), on);
                chip(ui, "3rd", on && organ.b3_perc_third());
            }
        });
    });
}

fn effects(ui: &mut Ui, p: &Electro5Program) {
    let fx = p.fx_panel();
    let extra = p.extra();

    card(ui, "effects", |ui| {
        fx_row(ui, "fx1", fx.fx1, &fx.fx1_type.to_string(), |ui| {
            meter(ui, "rate", fx.fx1_rate);
            chip(ui, "control", extra.fx1_control);
        });
        fx_row(ui, "fx2", fx.fx2, &fx.fx2_type.to_string(), |ui| {
            meter(ui, "rate", fx.fx2_rate);
            chip(ui, "deep", extra.fx2_deep);
        });
        fx_row(ui, "fx3", fx.fx3, &fx.fx3_type.to_string(), |ui| {
            meter(ui, "comp", fx.fx3_compression);
            chip(
                ui,
                if fx.rotary_speed {
                    "rotary fast"
                } else {
                    "rotary slow"
                },
                fx.rotary_speed,
            );
            chip(ui, "rotary stop", fx.rotary_stop);
        });
        fx_row(
            ui,
            "delay",
            fx.fx4,
            &format!("feedback {}", fx.fx4_feedback),
            |ui| {
                meter(ui, "tempo", fx.fx4_tempo);
                meter(ui, "wet", fx.fx4_moisture);
                chip(ui, "ping-pong", fx.fx4_ping_pong);
            },
        );

        ui.separator();

        ui.horizontal(|ui| {
            chip(ui, "reverb", fx.fx5);
            if fx.fx5 {
                ui.label(RichText::new(fx.fx5_type.to_string()).strong());
                meter(ui, "wet", fx.fx5_moisture);
            }
        });
        ui.horizontal_wrapped(|ui| {
            chip(ui, "eq", fx.equalizer_on);
            if fx.equalizer_on {
                ui.label(RichText::new(extra.equalizer_part.to_string()).strong());
                meter(ui, "bass", fx.equalizer_bass);
                meter(ui, "freq", fx.equalizer_freq);
                meter(ui, "gain", fx.equalizer_freq_gain);
                meter(ui, "treble", fx.equalizer_treble);
            }
        });
    });
}

/// One effect row: the routing badge decides whether the rest is even engaged.
fn fx_row(
    ui: &mut Ui,
    name: &str,
    routing: nord_format::electro5::Routing,
    kind: &str,
    engaged: impl FnOnce(&mut Ui),
) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(name).monospace().color(theme::DIM));
        match routing.part() {
            Some(part) => {
                chip(ui, part, true);
                ui.label(RichText::new(kind).strong());
                engaged(ui);
            }
            None => {
                chip(ui, &routing.to_string(), false);
            }
        }
    });
}

// ── widgets ─────────────────────────────────────────────────────────────────────

/// Nine drawbars, drawn the way they sit on the instrument: pulled *down* as the stored
/// position rises, coloured in the Hammond brown/white/black groups.
fn drawbars(ui: &mut Ui, bars: [u8; 9], count: usize) {
    const LABELS: [&str; 9] = ["16'", "5⅓'", "8'", "4'", "2⅔'", "2'", "1⅗'", "1⅓'", "1'"];
    const W: f32 = 30.0;
    const GAP: f32 = 8.0;
    const TRACK: f32 = 128.0;
    const KNOB: f32 = 22.0;

    let width = count as f32 * (W + GAP);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, TRACK + 20.0), Sense::hover());
    let painter = ui.painter();

    for (i, &value) in bars.iter().take(count).enumerate() {
        let x = rect.left() + i as f32 * (W + GAP);
        let track = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(W, TRACK));
        painter.rect_filled(track, CornerRadius::same(4), Color32::from_gray(18));

        // The stem the knob hangs from.
        let knob_top = rect.top() + (TRACK - KNOB) * (value as f32 / 8.0);
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(x + W * 0.5 - 2.0, rect.top() + 4.0),
                Pos2::new(x + W * 0.5 + 2.0, knob_top + KNOB * 0.5),
            ),
            CornerRadius::same(2),
            Color32::from_gray(40),
        );

        let (fill, ink) = drawbar_colors(i);
        let knob = Rect::from_min_size(Pos2::new(x, knob_top), Vec2::new(W, KNOB));
        painter.rect_filled(knob, CornerRadius::same(4), fill);
        painter.text(
            knob.center(),
            Align2::CENTER_CENTER,
            value.to_string(),
            FontId::monospace(12.0),
            ink,
        );
        painter.text(
            Pos2::new(x + W * 0.5, rect.top() + TRACK + 10.0),
            Align2::CENTER_CENTER,
            LABELS[i],
            FontId::proportional(11.0),
            theme::DIM,
        );
    }
}

/// The console colouring: 16' and 5⅓' brown, the mutations black, the rest white.
fn drawbar_colors(index: usize) -> (Color32, Color32) {
    match index {
        0 | 1 => (Color32::from_rgb(0x7c, 0x59, 0x36), Color32::from_gray(240)),
        4 | 6 | 7 => (Color32::from_rgb(0x24, 0x24, 0x28), Color32::from_gray(200)),
        _ => (Color32::from_rgb(0xe4, 0xe0, 0xd6), Color32::from_gray(20)),
    }
}

/// A 0..127 value as the panel's 0..10 bar, with the stored byte kept in the tooltip —
/// the file's precision is finer than the knob's, and rounding it away loses real data.
fn meter(ui: &mut Ui, label: &str, level: Level) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(theme::DIM).size(11.0));
        let (rect, response) = ui.allocate_exact_size(Vec2::new(84.0, 9.0), Sense::hover());
        {
            let painter = ui.painter();
            painter.rect_filled(rect, CornerRadius::same(4), Color32::from_gray(22));
            let filled = rect.width() * (level.as_u8() as f32 / Level::MAX as f32);
            painter.rect_filled(
                Rect::from_min_size(rect.min, Vec2::new(filled, rect.height())),
                CornerRadius::same(4),
                theme::RED,
            );
        }
        response.on_hover_text(format!("stored {}", level.as_u8()));
        ui.label(
            RichText::new(format!("{:.1}", level.as_panel()))
                .monospace()
                .size(11.0),
        );
    });
}

fn chip(ui: &mut Ui, text: &str, on: bool) {
    let (fill, ink) = if on {
        (theme::RED, Color32::from_gray(245))
    } else {
        (Color32::from_gray(34), theme::DIM)
    };
    egui::Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(ink));
        });
}

fn kv(ui: &mut Ui, key: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(RichText::new(key).color(theme::DIM).size(11.0));
        ui.label(RichText::new(value).size(14.0).strong());
    });
}

fn card(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui)) {
    theme::card_ui(ui, |ui| {
        // Cards take the full width of whatever they are in — the page, or one column.
        ui.set_width(ui.available_width());
        ui.label(
            RichText::new(title.to_uppercase())
                .size(11.0)
                .color(theme::RED_TEXT)
                .strong(),
        );
        ui.add_space(6.0);
        body(ui);
    });
}

/// The library id a program depends on, in the hex `nord device deps` reports.
fn dep_id(id: u32) -> String {
    match id {
        0 => "none".to_string(),
        id => format!("{id:#010x}"),
    }
}
