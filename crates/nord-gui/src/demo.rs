//! A program built in memory, so the demo has something on screen with no corpus and
//! no instrument attached.
//!
//! It is assembled with the library's own setters and then **encoded to `.ne5p` bytes**,
//! which the app parses back through the ordinary load path. So this is not a shortcut
//! around the format: what you see rendered came out of a real parse, and the
//! round-trip badge in the title bar is checking real bytes.

use nord_format::electro5::program::{Location, OrganModel, PercSpeed, Program, VibChorus};
use nord_format::electro5::{
    Fx1Type, Fx2Type, Fx3Type, Fx5Type, Instrument, Level, OrganType, PianoCategory, Routing,
};
use nord_format::{Entity, Program as AnyProgram};

pub const NAME: &str = "demo.ne5p";

/// A gospel-ish B3 registration, split, with rotary and a hall reverb.
pub fn bytes() -> Vec<u8> {
    let mut p = Program::new(Location::new(6, 3).expect("bank 6 slot 3 exists"));

    let c = &mut p.schema.center_panel;
    c.left_part = Instrument::Piano;
    c.right_part = Instrument::Organ;
    c.left_octave_shift = 0i8.try_into().expect("0 is in range");
    c.right_octave_shift = 1i8.try_into().expect("+1 is in range");
    c.left_sustain = true;
    c.right_control = true;
    c.split = true;
    c.split_point = nord_format::electro5::SplitPoint::C4;
    c.transpose = 0i8.try_into().expect("0 is in range");
    c.transpose_enabled = false;
    c.part_mix = 64u8.try_into().expect("64 is in range");
    c.gain = Level::new(96).expect("96 is in range");
    c.organ_type = OrganType::B3;
    c.lower_enabled = true;
    c.upper_enabled = true;
    c.drawbar_live = true;

    let piano = &mut p.schema.piano_panel;
    piano.category = PianoCategory::EPiano1;
    piano.piano_model = 2u8.try_into().expect("model 2 is in range");
    piano.acoustics = 1u8.try_into().expect("1 is in range");
    piano.touch = 2u8.try_into().expect("2 is in range");

    let s = &mut p.schema.sample_panel;
    s.number = 17;
    s.attack = Level::new(0).expect("0 is in range");
    s.decay_release = Level::new(64).expect("64 is in range");

    let o = &mut p.schema.organ_panel;
    // The classic 888000000 on preset 1, a thinner 800808000 on preset 2.
    o.set_drawbars(OrganModel::B3, 1, [8, 8, 8, 0, 0, 0, 0, 0, 0])
        .expect("positions are 0..=8");
    o.set_drawbars(OrganModel::B3, 2, [8, 0, 0, 8, 0, 8, 0, 0, 0])
        .expect("positions are 0..=8");
    o.set_drawbars(OrganModel::Vox, 1, [8, 6, 4, 0, 0, 4, 0, 0, 8])
        .expect("positions are 0..=8");
    o.set_drawbars(OrganModel::Pipe, 1, [8, 0, 8, 0, 8, 0, 0, 0, 4])
        .expect("positions are 0..=8");
    o.set_farfisa_tabs(
        1,
        [true, false, true, true, false, false, false, false, true],
    );
    o.set_preset(OrganModel::B3, 1);
    o.set_vib_on(OrganModel::B3, 1, true);
    o.set_vib_type(OrganModel::B3, VibChorus::C3)
        .expect("c3 exists on the b3");
    o.set_b3_perc_on(1, true);
    o.set_b3_perc_third(true);
    o.set_b3_perc_speed(PercSpeed::Fast);

    let fx = &mut p.schema.effects_panel;
    fx.fx1 = Routing::Lower;
    fx.fx1_type = Fx1Type::Trem1;
    fx.fx1_rate = Level::new(58).expect("58 is in range");
    fx.fx2 = Routing::Upper;
    fx.fx2_type = Fx2Type::Vibe;
    fx.fx2_rate = Level::new(40).expect("40 is in range");
    fx.fx3 = Routing::Upper;
    fx.fx3_type = Fx3Type::Rotary;
    fx.fx3_compression = Level::new(0).expect("0 is in range");
    fx.fx4 = Routing::Lower;
    fx.fx4_tempo = Level::new(72).expect("72 is in range");
    fx.fx4_moisture = Level::new(38).expect("38 is in range");
    fx.fx4_feedback = 2u8.try_into().expect("2 is in range");
    fx.fx4_ping_pong = true;
    fx.fx5 = true;
    fx.fx5_type = Fx5Type::Hall;
    fx.fx5_moisture = Level::new(52).expect("52 is in range");
    fx.equalizer_on = true;
    fx.equalizer_bass = Level::new(80).expect("80 is in range");
    fx.equalizer_freq = Level::new(64).expect("64 is in range");
    fx.equalizer_freq_gain = Level::new(70).expect("70 is in range");
    fx.equalizer_treble = Level::new(90).expect("90 is in range");
    fx.rotary_speed = true;

    p.schema.extra.fx1_control = true;
    p.schema.extra.fx2_deep = true;
    p.schema.extra.equalizer_part = nord_format::electro5::EqualizerPart::Upper;

    let mut entity = Entity::Program(AnyProgram::Electro5(p));
    nord_format::to_bytes(&mut entity).expect("a program built from typed values encodes")
}
