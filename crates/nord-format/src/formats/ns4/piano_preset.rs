//! The Stage 4 piano preset body (`.ns4n`): 151 bytes, 165 parameters.
//!
//! One piano section as a program stores it, moved down 180 bytes, without the
//! keyboard zone — that belongs to the program that loads the preset.

use crate::cbin::{self, Cbin};
use crate::error::Error;
use crate::types::RangedU8;
use std::io::{Read, Seek};

pub const FORMAT: &str = "ns4n";
/// Stored ×100. The corpus holds 2.03; ns4decode was tested on 2.01.
pub const KNOWN_VERSIONS: &[u32] = &[201, 202, 203];
pub const BODY_LEN: usize = 151;

#[nord_bits_derive::bitbody(151)]
pub struct PianoPreset {
    #[bits(41..=41)]
    pub piano_b_layer_enabled: bool,
    #[bits(42..=42)]
    pub piano_a_layer_enabled: bool,
    #[bits(44..=44)]
    pub piano_b_layer_enabled_scene_2: bool,
    #[bits(45..=45)]
    pub piano_a_layer_enabled_scene_2: bool,
    #[bits(46..=52)]
    pub piano_a_volume: RangedU8<127>,
    #[bits(53..=60)]
    pub piano_a_volume_wheel: u8,
    #[bits(61..=68)]
    pub piano_a_volume_aftertouch: u8,
    #[bits(69..=76)]
    pub piano_a_volume_ctrl_pedal: u8,
    #[bits(77..=83)]
    pub piano_b_volume: RangedU8<127>,
    #[bits(84..=91)]
    pub piano_b_volume_wheel: u8,
    #[bits(92..=99)]
    pub piano_b_volume_aftertouch: u8,
    #[bits(100..=107)]
    pub piano_b_volume_ctrl_pedal: u8,
    #[bits(148..=151)]
    pub piano_a_octave_shift: RangedU8<15>,
    #[bits(152..=152)]
    pub piano_a_pitch_stick_enabled: bool,
    #[bits(153..=153)]
    pub piano_a_sustain_pedal_enabled: bool,
    #[bits(154..=156)]
    pub piano_a_type: RangedU8<7>,
    #[bits(157..=161)]
    pub piano_a_model_slot: RangedU8<31>,
    #[bits(162..=163)]
    pub piano_a_model_variation: RangedU8<3>,
    #[bits(164..=195)]
    pub piano_a_model_id: u32,
    #[bits(196..=196)]
    pub piano_a_soft_rel_enabled: bool,
    #[bits(197..=197)]
    pub piano_a_string_res_enabled: bool,
    #[bits(198..=198)]
    pub piano_a_pedal_noise_enabled: bool,
    #[bits(199..=200)]
    pub piano_a_touch: RangedU8<3>,
    #[bits(201..=202)]
    pub piano_a_unison_level: RangedU8<3>,
    #[bits(203..=204)]
    pub piano_a_dyn_comp: RangedU8<3>,
    #[bits(206..=208)]
    pub piano_a_timbre: RangedU8<7>,
    #[bits(240..=243)]
    pub piano_b_kb_zones: RangedU8<15>,
    #[bits(244..=247)]
    pub piano_b_octave_shift: RangedU8<15>,
    #[bits(248..=248)]
    pub piano_b_pitch_stick_enabled: bool,
    #[bits(249..=249)]
    pub piano_b_sustain_pedal_enabled: bool,
    #[bits(250..=252)]
    pub piano_b_type: RangedU8<7>,
    #[bits(253..=257)]
    pub piano_b_model_slot: RangedU8<31>,
    #[bits(258..=259)]
    pub piano_b_model_variation: RangedU8<3>,
    #[bits(260..=291)]
    pub piano_b_model_id: u32,
    #[bits(292..=292)]
    pub piano_b_soft_rel_enabled: bool,
    #[bits(293..=293)]
    pub piano_b_string_res_enabled: bool,
    #[bits(294..=294)]
    pub piano_b_pedal_noise_enabled: bool,
    #[bits(295..=296)]
    pub piano_b_touch: RangedU8<3>,
    #[bits(297..=298)]
    pub piano_b_unison_level: RangedU8<3>,
    #[bits(299..=300)]
    pub piano_b_dyn_comp: RangedU8<3>,
    #[bits(302..=304)]
    pub piano_b_timbre: RangedU8<7>,
    #[bits(336..=336)]
    pub piano_a_fx_mod_1_enabled: bool,
    #[bits(337..=337)]
    pub piano_a_fx_mod_1_master_clock_enabled: bool,
    #[bits(338..=344)]
    pub piano_a_fx_mod_1_rate: RangedU8<127>,
    #[bits(345..=352)]
    pub piano_a_fx_mod_1_rate_wheel: u8,
    #[bits(353..=360)]
    pub piano_a_fx_mod_1_rate_aftertouch: u8,
    #[bits(361..=368)]
    pub piano_a_fx_mod_1_rate_ctrl_pedal: u8,
    #[bits(369..=375)]
    pub piano_a_fx_mod_1_amount: RangedU8<127>,
    #[bits(376..=383)]
    pub piano_a_fx_mod_1_amount_wheel: u8,
    #[bits(384..=391)]
    pub piano_a_fx_mod_1_amount_aftertouch: u8,
    #[bits(392..=399)]
    pub piano_a_fx_mod_1_amount_ctrl_pedal: u8,
    #[bits(400..=403)]
    pub piano_a_fx_mod_1_mode: RangedU8<15>,
    #[bits(404..=404)]
    pub piano_a_fx_mod_2_enabled: bool,
    #[bits(405..=411)]
    pub piano_a_fx_mod_2_rate: RangedU8<127>,
    #[bits(412..=419)]
    pub piano_a_fx_mod_2_rate_wheel: u8,
    #[bits(420..=427)]
    pub piano_a_fx_mod_2_rate_aftertouch: u8,
    #[bits(428..=435)]
    pub piano_a_fx_mod_2_rate_ctrl_pedal: u8,
    #[bits(436..=442)]
    pub piano_a_fx_mod_2_amount: RangedU8<127>,
    #[bits(443..=450)]
    pub piano_a_fx_mod_2_amount_wheel: u8,
    #[bits(451..=458)]
    pub piano_a_fx_mod_2_amount_aftertouch: u8,
    #[bits(459..=466)]
    pub piano_a_fx_mod_2_amount_ctrl_pedal: u8,
    #[bits(467..=470)]
    pub piano_a_fx_mod_2_mode: RangedU8<15>,
    #[bits(471..=471)]
    pub piano_a_fx_amp_sim_eq_enabled: bool,
    #[bits(472..=478)]
    pub piano_a_fx_amp_sim_eq_treb: RangedU8<127>,
    #[bits(479..=485)]
    pub piano_a_fx_amp_sim_eq_mid: RangedU8<127>,
    #[bits(486..=492)]
    pub piano_a_fx_amp_sim_eq_bass: RangedU8<127>,
    #[bits(493..=499)]
    pub piano_a_fx_amp_sim_eq_freq: RangedU8<127>,
    #[bits(500..=507)]
    pub piano_a_fx_amp_sim_eq_freq_wheel: u8,
    #[bits(508..=515)]
    pub piano_a_fx_amp_sim_eq_freq_aftertouch: u8,
    #[bits(516..=523)]
    pub piano_a_fx_amp_sim_eq_freq_ctrl_pedal: u8,
    #[bits(524..=530)]
    pub piano_a_fx_amp_sim_eq_drive: RangedU8<127>,
    #[bits(531..=538)]
    pub piano_a_fx_amp_sim_eq_drive_wheel: u8,
    #[bits(539..=546)]
    pub piano_a_fx_amp_sim_eq_drive_aftertouch: u8,
    #[bits(547..=554)]
    pub piano_a_fx_amp_sim_eq_drive_ctrl_pedal: u8,
    #[bits(559..=559)]
    pub piano_a_fx_comp_enabled: bool,
    #[bits(560..=566)]
    pub piano_a_fx_comp_amount: RangedU8<127>,
    #[bits(567..=567)]
    pub piano_a_fx_comp_response: bool,
    #[bits(568..=568)]
    pub piano_a_fx_delay_enabled: bool,
    #[bits(569..=569)]
    pub piano_a_fx_delay_tempo_master_clock_enabled: bool,
    #[bits(570..=576)]
    pub piano_a_fx_delay_tempo: RangedU8<127>,
    #[bits(584..=591)]
    pub piano_a_fx_delay_tempo_wheel: u8,
    #[bits(592..=599)]
    pub piano_a_fx_delay_tempo_aftertouch: u8,
    #[bits(600..=607)]
    pub piano_a_fx_delay_tempo_ctrl_pedal: u8,
    #[bits(629..=635)]
    pub piano_a_fx_delay_mix: RangedU8<127>,
    #[bits(636..=643)]
    pub piano_a_fx_delay_mix_wheel: u8,
    #[bits(644..=651)]
    pub piano_a_fx_delay_mix_aftertouch: u8,
    #[bits(652..=659)]
    pub piano_a_fx_delay_mix_ctrl_pedal: u8,
    #[bits(660..=660)]
    pub piano_a_fx_delay_normal_analog: bool,
    #[bits(661..=661)]
    pub piano_a_fx_delay_ping_pong_enabled: bool,
    #[bits(662..=663)]
    pub piano_a_fx_delay_filter_type: RangedU8<3>,
    #[bits(664..=670)]
    pub piano_a_fx_delay_feedback: RangedU8<127>,
    #[bits(671..=678)]
    pub piano_a_fx_delay_feedback_wheel: u8,
    #[bits(679..=686)]
    pub piano_a_fx_delay_feedback_aftertouch: u8,
    #[bits(687..=694)]
    pub piano_a_fx_delay_feedback_ctrl_pedal: u8,
    #[bits(695..=698)]
    pub piano_a_fx_delay_effects: RangedU8<15>,
    #[bits(699..=699)]
    pub piano_a_fx_reverb_enabled: bool,
    #[bits(700..=706)]
    pub piano_a_fx_reverb_amount: RangedU8<127>,
    #[bits(707..=714)]
    pub piano_a_fx_reverb_amount_wheel: u8,
    #[bits(715..=722)]
    pub piano_a_fx_reverb_amount_aftertouch: u8,
    #[bits(723..=730)]
    pub piano_a_fx_reverb_amount_ctrl_pedal: u8,
    #[bits(731..=732)]
    pub piano_a_fx_reverb_dark_bright: RangedU8<3>,
    #[bits(733..=736)]
    pub piano_a_fx_reverb_type: RangedU8<15>,
    #[bits(741..=744)]
    pub piano_a_fx_amp_sim_eq_mode: RangedU8<15>,
    #[bits(776..=776)]
    pub piano_b_fx_mod_1_enabled: bool,
    #[bits(777..=777)]
    pub piano_b_fx_mod_1_master_clock_enabled: bool,
    #[bits(778..=784)]
    pub piano_b_fx_mod_1_rate: RangedU8<127>,
    #[bits(785..=792)]
    pub piano_b_fx_mod_1_rate_wheel: u8,
    #[bits(793..=800)]
    pub piano_b_fx_mod_1_rate_aftertouch: u8,
    #[bits(801..=808)]
    pub piano_b_fx_mod_1_rate_ctrl_pedal: u8,
    #[bits(809..=815)]
    pub piano_b_fx_mod_1_amount: RangedU8<127>,
    #[bits(816..=823)]
    pub piano_b_fx_mod_1_amount_wheel: u8,
    #[bits(824..=831)]
    pub piano_b_fx_mod_1_amount_aftertouch: u8,
    #[bits(832..=839)]
    pub piano_b_fx_mod_1_amount_ctrl_pedal: u8,
    #[bits(840..=843)]
    pub piano_b_fx_mod_1_mode: RangedU8<15>,
    #[bits(844..=844)]
    pub piano_b_fx_mod_2_enabled: bool,
    #[bits(845..=851)]
    pub piano_b_fx_mod_2_rate: RangedU8<127>,
    #[bits(852..=859)]
    pub piano_b_fx_mod_2_rate_wheel: u8,
    #[bits(860..=867)]
    pub piano_b_fx_mod_2_rate_aftertouch: u8,
    #[bits(868..=875)]
    pub piano_b_fx_mod_2_rate_ctrl_pedal: u8,
    #[bits(876..=882)]
    pub piano_b_fx_mod_2_amount: RangedU8<127>,
    #[bits(883..=890)]
    pub piano_b_fx_mod_2_amount_wheel: u8,
    #[bits(891..=898)]
    pub piano_b_fx_mod_2_amount_aftertouch: u8,
    #[bits(899..=906)]
    pub piano_b_fx_mod_2_amount_ctrl_pedal: u8,
    #[bits(907..=910)]
    pub piano_b_fx_mod_2_mode: RangedU8<15>,
    #[bits(911..=911)]
    pub piano_b_fx_amp_sim_eq_enabled: bool,
    #[bits(912..=918)]
    pub piano_b_fx_amp_sim_eq_treb: RangedU8<127>,
    #[bits(919..=925)]
    pub piano_b_fx_amp_sim_eq_mid: RangedU8<127>,
    #[bits(926..=932)]
    pub piano_b_fx_amp_sim_eq_bass: RangedU8<127>,
    #[bits(933..=939)]
    pub piano_b_fx_amp_sim_eq_freq: RangedU8<127>,
    #[bits(940..=947)]
    pub piano_b_fx_amp_sim_eq_freq_wheel: u8,
    #[bits(948..=955)]
    pub piano_b_fx_amp_sim_eq_freq_aftertouch: u8,
    #[bits(956..=963)]
    pub piano_b_fx_amp_sim_eq_freq_ctrl_pedal: u8,
    #[bits(964..=970)]
    pub piano_b_fx_amp_sim_eq_drive: RangedU8<127>,
    #[bits(971..=978)]
    pub piano_b_fx_amp_sim_eq_drive_wheel: u8,
    #[bits(979..=986)]
    pub piano_b_fx_amp_sim_eq_drive_aftertouch: u8,
    #[bits(987..=994)]
    pub piano_b_fx_amp_sim_eq_drive_ctrl_pedal: u8,
    #[bits(999..=999)]
    pub piano_b_fx_comp_enabled: bool,
    #[bits(1000..=1006)]
    pub piano_b_fx_comp_amount: RangedU8<127>,
    #[bits(1007..=1007)]
    pub piano_b_fx_comp_response: bool,
    #[bits(1008..=1008)]
    pub piano_b_fx_delay_enabled: bool,
    #[bits(1009..=1009)]
    pub piano_b_fx_delay_tempo_master_clock_enabled: bool,
    #[bits(1010..=1016)]
    pub piano_b_fx_delay_tempo: RangedU8<127>,
    #[bits(1024..=1031)]
    pub piano_b_fx_delay_tempo_wheel: u8,
    #[bits(1032..=1039)]
    pub piano_b_fx_delay_tempo_aftertouch: u8,
    #[bits(1040..=1047)]
    pub piano_b_fx_delay_tempo_ctrl_pedal: u8,
    #[bits(1069..=1075)]
    pub piano_b_fx_delay_mix: RangedU8<127>,
    #[bits(1076..=1083)]
    pub piano_b_fx_delay_mix_wheel: u8,
    #[bits(1084..=1091)]
    pub piano_b_fx_delay_mix_aftertouch: u8,
    #[bits(1092..=1099)]
    pub piano_b_fx_delay_mix_ctrl_pedal: u8,
    #[bits(1100..=1100)]
    pub piano_b_fx_delay_normal_analog: bool,
    #[bits(1101..=1101)]
    pub piano_b_fx_delay_ping_pong_enabled: bool,
    #[bits(1102..=1103)]
    pub piano_b_fx_delay_filter_type: RangedU8<3>,
    #[bits(1104..=1110)]
    pub piano_b_fx_delay_feedback: RangedU8<127>,
    #[bits(1111..=1118)]
    pub piano_b_fx_delay_feedback_wheel: u8,
    #[bits(1119..=1126)]
    pub piano_b_fx_delay_feedback_aftertouch: u8,
    #[bits(1127..=1134)]
    pub piano_b_fx_delay_feedback_ctrl_pedal: u8,
    #[bits(1135..=1138)]
    pub piano_b_fx_delay_effects: RangedU8<15>,
    #[bits(1139..=1139)]
    pub piano_b_fx_reverb_enabled: bool,
    #[bits(1140..=1146)]
    pub piano_b_fx_reverb_amount: RangedU8<127>,
    #[bits(1147..=1154)]
    pub piano_b_fx_reverb_amount_wheel: u8,
    #[bits(1155..=1162)]
    pub piano_b_fx_reverb_amount_aftertouch: u8,
    #[bits(1163..=1170)]
    pub piano_b_fx_reverb_amount_ctrl_pedal: u8,
    #[bits(1171..=1172)]
    pub piano_b_fx_reverb_dark_bright: RangedU8<3>,
    #[bits(1173..=1176)]
    pub piano_b_fx_reverb_type: RangedU8<15>,
    #[bits(1181..=1184)]
    pub piano_b_fx_amp_sim_eq_mode: RangedU8<15>,
}

pub fn read_from(reader: &mut (impl Read + Seek)) -> Result<Cbin<PianoPreset>, Error> {
    let file: Cbin<PianoPreset> = cbin::read(reader, FORMAT)?;
    crate::formats::known_version(FORMAT, file.header.version, KNOWN_VERSIONS)?;
    Ok(file)
}
