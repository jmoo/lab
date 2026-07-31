//! The human rendering of a decoded entity.
//!
//! Everything here is *data*, so it goes to stdout: `nord inspect x.ne5p | grep transpose`
//! has to work.

use nord_format::common::bank::Item;
use nord_format::electro5::{Instrument, OrganModel};
use nord_format::{Entity, Program, Settings, Song};

use crate::ui::Ui;

/// One-indexed `bank N slot M` — matches how the hardware labels locations.
fn location(x: u16, y: u16) -> String {
    format!("bank {} slot {}", x + 1, y + 1)
}

fn yn(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}

/// Format a library dependency id the way it is worth reading: hex, matching what
/// `nord program deps` reports for the same program.
fn dep_id(id: u32) -> String {
    match id {
        0 => "none".to_string(),
        id => format!("{id:#010x}"),
    }
}

pub fn print(ui: &Ui, entity: &Entity) {
    match entity {
        Entity::Program(Program::Electro5(p)) => {
            let l = p.location();
            let split = if p.schema.center_panel.split {
                format!("yes @ {:?}", p.schema.center_panel.split_point)
            } else {
                "no".to_string()
            };
            ui.out("  type:      Electro 5 program (ne5p)");
            ui.out(format!("  location:  {}", location(l.x(), l.y())));
            ui.out(format!(
                "  lower:     {:?}  octave {:+}  sustain {}  control {}",
                p.schema.center_panel.lower_part,
                p.schema.center_panel.lower_octave_shift.inner(),
                yn(p.schema.center_panel.lower_sustain),
                yn(p.schema.center_panel.lower_control),
            ));
            ui.out(format!(
                "  upper:     {:?}  octave {:+}  sustain {}  control {}",
                p.schema.center_panel.upper_part,
                p.schema.center_panel.upper_octave_shift.inner(),
                yn(p.schema.center_panel.upper_sustain),
                yn(p.schema.center_panel.upper_control),
            ));
            ui.out(format!("  split:     {split}"));
            ui.out(format!(
                "  transpose: {:+}  ({})",
                p.schema.center_panel.transpose.inner(),
                yn(p.schema.center_panel.transpose_enabled),
            ));
            ui.out(format!(
                "  part mix:  {} (lower/upper %)",
                p.schema.center_panel.part_mix.as_string()
            ));
            ui.out(format!("  gain:      {}", p.schema.center_panel.gain));

            let (piano, sample) = (&p.schema.piano_panel, &p.schema.sample_panel);
            ui.out(format!(
                "  piano:     category {}  model {}  clav {}  acoustics {}  touch {}  mono {}",
                piano.category,
                piano.piano_model.as_u8(),
                piano.clav_model.as_u8(),
                piano.acoustics.as_u8(),
                piano.touch.as_u8(),
                yn(piano.mono),
            ));
            ui.out(format!(
                "  sample:    number {}  attack {}  decay/rel {}  dynamics {}  filter {}",
                sample.number,
                sample.attack,
                sample.decay_release,
                sample.dynamics.as_u8(),
                yn(sample.filter),
            ));
            // The two library references. `nord program deps` reports these same ids
            // for this program with the piano's and sample's *names* attached — the
            // file itself stores only the id, so that is the only way to resolve them.
            ui.out(format!(
                "  depends:   piano {}  sample {}",
                dep_id(piano.id),
                dep_id(sample.id),
            ));

            // Effects. Values are printed as stored (0..127); the panel shows most of
            // them on a 0..10 scale, and the two do not map linearly for delay tempo,
            // so rescaling here would invent precision the file does not carry.
            let fx = &p.schema.effects_panel;
            ui.out("  fx:        stored value, with the panel's 0-10 reading where it applies");
            match fx.fx1.part() {
                Some(part) => ui.out(format!(
                    "    fx1   {part:<5}  {:<9}  rate {}  control {}",
                    fx.fx1_type,
                    fx.fx1_rate,
                    yn(fx.fx1_control),
                )),
                None => ui.out(format!("    fx1   {}", fx.fx1)),
            }
            match fx.fx2.part() {
                Some(part) => ui.out(format!(
                    "    fx2   {part:<5}  {:<9}  rate {}  deep {}",
                    fx.fx2_type,
                    fx.fx2_rate.as_u8(),
                    yn(fx.fx2_deep),
                )),
                None => ui.out(format!("    fx2   {}", fx.fx2)),
            }
            match fx.fx3.part() {
                Some(part) => ui.out(format!(
                    "    fx3   {part:<5}  {:<9}  compression {}",
                    fx.fx3_type, fx.fx3_compression,
                )),
                None => ui.out(format!("    fx3   {}", fx.fx3)),
            }
            match fx.fx4.part() {
                Some(part) => ui.out(format!(
                    "    delay {part:<5}  feedback {}  tempo {}  wet {}  ping-pong {}",
                    fx.fx4_feedback.as_u8(),
                    fx.fx4_tempo.as_u8(),
                    fx.fx4_moisture,
                    yn(fx.fx4_ping_pong),
                )),
                None => ui.out(format!("    delay {}", fx.fx4)),
            }
            // ⚠️ The reverb enable bit reads `true` in every `fx5_1xx` specimen, and the
            // corpus README's "0: on, 1: off" would make all of them captures of a
            // *disabled* reverb whose type and wet level were then varied — which would
            // be pointless. Rendered as on/off accordingly; worth one confirmation
            // against the panel.
            if fx.fx5 {
                ui.out(format!(
                    "    reverb {:<9}  wet {}",
                    fx.fx5_type, fx.fx5_moisture,
                ));
            } else {
                ui.out("    reverb off");
            }
            // `0` for the EQ routing means *lower*, not off, so the enable has to be
            // checked first.
            if fx.equalizer_on {
                let part = fx.equalizer_part;
                ui.out(format!(
                    "    eq    {part:<11}  bass {}  freq {}  gain {}  treble {}",
                    fx.equalizer_bass.as_u8(),
                    fx.equalizer_freq.as_u8(),
                    fx.equalizer_freq_gain.as_u8(),
                    fx.equalizer_treble.as_u8(),
                ));
            } else {
                ui.out("    eq    off");
            }
            ui.out(format!(
                "    rotary speed {}  stop {}",
                if fx.rotary_speed { "fast" } else { "slow" },
                if fx.rotary_stop { "on" } else { "off" },
            ));

            // Organ. Both presets are shown for every model: the file keeps the full
            // state of all four, and b3+bass cannot be checked at all without seeing
            // preset 1 and preset 2 side by side.
            if p.schema.center_panel.lower_part == Instrument::Organ
                || p.schema.center_panel.upper_part == Instrument::Organ
            {
                let o = &p.schema.organ_panel;
                let selected = p.schema.center_panel.organ_type;
                // b3+bass shares the B3's storage, so it marks the B3 rows.
                let sel_model = selected.storage();
                ui.out(format!(
                    "  organ:     {selected} selected (*), drawbar positions 0-8"
                ));
                for (model, label) in [
                    (OrganModel::B3, "b3"),
                    (OrganModel::Vox, "vox"),
                    (OrganModel::Farfisa, "farf"),
                    (OrganModel::Pipe, "pipe"),
                ] {
                    for preset in 1..=2u8 {
                        let mark = if Some(model) == sel_model { "*" } else { " " };
                        let live = if o.preset(model) == preset { "<" } else { " " };

                        // In b3+bass, preset 1 is the bass manual: two drawbars, kept
                        // outside the nine-nibble block. The nine nibbles are stale
                        // there, so printing them would be actively misleading.
                        let bars =
                            if selected.is_b3_bass() && model == OrganModel::B3 && preset == 1 {
                                // Only two of the nine positions exist on the bass manual;
                                // dots for the rest so it lines up with the other rows and
                                // cannot be misread as a nine-drawbar registration.
                                let b = o.b3_bass_drawbars();
                                format!("{}{}.......", b[0], b[1])
                            } else if model == OrganModel::Farfisa {
                                // Farfisa's drawbars are on/off tabs on the panel, but the
                                // file still stores nine positions and the low bits of each
                                // vary independently of the on/off threshold. Show both, or
                                // the display silently discards them.
                                let tabs: String = o
                                    .farfisa_tabs(preset)
                                    .iter()
                                    .map(|on| if *on { '|' } else { '.' })
                                    .collect();
                                let pos: String = o
                                    .drawbars(model, preset)
                                    .iter()
                                    .map(u8::to_string)
                                    .collect();
                                format!("{tabs} ({pos})")
                            } else {
                                o.drawbars(model, preset)
                                    .iter()
                                    .map(u8::to_string)
                                    .collect()
                            };

                        let vib = match o.vib_type(model) {
                            Some(v) if o.vib_on(model, preset) => format!("  vib {v:?}"),
                            Some(_) => "  vib off".to_string(),
                            None => String::new(),
                        };
                        let perc = if model == OrganModel::B3 {
                            if o.b3_perc_on(preset) {
                                let third = if o.b3_perc_third() { " +3rd" } else { "" };
                                format!("  perc {:?}{third}", o.b3_perc_speed())
                            } else {
                                "  perc off".to_string()
                            }
                        } else {
                            String::new()
                        };
                        ui.out(format!(
                            "   {mark}{label:<5} p{preset}{live} {bars}{vib}{perc}"
                        ));
                    }
                }
                ui.out("   (* = selected model, < = its active preset)");
            }
        }
        Entity::Song(Song::Electro5(s)) => {
            let l = s.location();
            ui.out("  type:      Electro 5 song / set (ne5t)");
            ui.out(format!("  location:  {}", location(l.x(), l.y())));
            for slot in 0..4u16 {
                let p = s.get(slot);
                ui.out(format!(
                    "    slot {}:  program {}",
                    slot + 1,
                    location(p.x(), p.y())
                ));
            }
        }
        Entity::Settings(Settings::Electro5(s)) => {
            ui.out("  type:      Electro 5 settings (ne5s)");
            ui.out("  note:      field decode pending specimens; raw body below");
            let hex: String = s
                .raw()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            ui.out(format!("  body:      {hex}"));
        }
        Entity::Piano(_) => {
            ui.out(format!(
                "  type:      piano (npno) {} header/reference only",
                ui.dash()
            ));
        }
        Entity::Sample(_) => {
            ui.out(format!(
                "  type:      sample (nsmp) {} header/reference only",
                ui.dash()
            ));
        }
        Entity::Bundle(nord_format::Bundle::Electro5(b)) => {
            ui.out("  type:      backup bundle (zip)");
            if let Some(name) = b.name() {
                ui.out(format!("  name:      {name}"));
            }
            ui.out("  note:      use --raw to list contained programs/songs");
            let _ = (b.programs(), b.songs()); // decoded; shown via --raw
        }
    }
}
