//! The Stage 4 program body (`.ns4p`, `.ns4l`): 824 bytes, 878 parameters.
//!
//! All three sections and the globals that route them. Placements, naming and
//! provenance are the [module docs](super); values are raw.

use crate::cbin::{self, Cbin};
use crate::error::Error;
use crate::types::{RangedU16, RangedU8};
use std::io::{Read, Seek};

pub const FORMAT: &str = "ns4p";
/// Schema versions this build's field offsets have been validated against,
/// stored ×100. The corpus holds 3.13 throughout; ns4decode reports the same
/// offsets back to 3.04, changing only how three values are *read*, which this
/// decode does not do.
pub const KNOWN_VERSIONS: &[u32] = &[304, 305, 306, 307, 308, 309, 310, 311, 312, 313];
pub const BODY_LEN: usize = 824;

#[nord_bits_derive::bitbody(824)]
pub struct Program {
    #[bits(24..=31)]
    pub version_echo: u8,
    #[bits(40..=40)]
    pub split_enabled: bool,
    #[bits(41..=41)]
    pub kb_zones_1_2_split_point_enabled: bool,
    #[bits(42..=42)]
    pub kb_zones_2_3_split_point_enabled: bool,
    #[bits(43..=43)]
    pub kb_zones_3_4_split_point_enabled: bool,
    #[bits(44..=47)]
    pub kb_zones_1_2_split_point: RangedU8<15>,
    #[bits(48..=51)]
    pub kb_zones_2_3_split_point: RangedU8<15>,
    #[bits(52..=55)]
    pub kb_zones_3_4_split_point: RangedU8<15>,
    #[bits(56..=57)]
    pub kb_zones_1_2_split_point_xfade: RangedU8<3>,
    #[bits(58..=59)]
    pub kb_zones_2_3_split_point_xfade: RangedU8<3>,
    #[bits(60..=61)]
    pub kb_zones_3_4_split_point_xfade: RangedU8<3>,
    #[bits(62..=62)]
    pub program_transpose_enabled: bool,
    #[bits(63..=66)]
    pub program_transpose_amount: RangedU8<15>,
    #[bits(79..=79)]
    pub fx_comp_global_enabled: bool,
    #[bits(80..=80)]
    pub fx_delay_global_enabled: bool,
    #[bits(81..=81)]
    pub fx_reverb_global_enabled: bool,
    #[bits(316..=316)]
    pub organ_section_enabled: bool,
    #[bits(317..=317)]
    pub piano_section_enabled: bool,
    #[bits(318..=318)]
    pub synth_section_enabled: bool,
    #[bits(319..=319)]
    pub organ_section_enabled_scene_2: bool,
    #[bits(320..=320)]
    pub piano_section_enabled_scene_2: bool,
    #[bits(321..=321)]
    pub synth_section_enabled_scene_2: bool,
    #[bits(322..=322)]
    pub active_layer_scene: bool,
    #[bits(347..=347)]
    pub fx_enabled: bool,
    #[bits(401..=401)]
    pub organ_b_layer_enabled: bool,
    #[bits(402..=402)]
    pub organ_a_layer_enabled: bool,
    #[bits(404..=404)]
    pub organ_b_layer_enabled_scene_2: bool,
    #[bits(405..=405)]
    pub organ_a_layer_enabled_scene_2: bool,
    #[bits(406..=412)]
    pub organ_a_volume: RangedU8<127>,
    #[bits(413..=420)]
    pub organ_a_volume_wheel: u8,
    #[bits(421..=428)]
    pub organ_a_volume_aftertouch: u8,
    #[bits(429..=436)]
    pub organ_a_volume_ctrl_pedal: u8,
    #[bits(437..=443)]
    pub organ_b_volume: RangedU8<127>,
    #[bits(444..=451)]
    pub organ_b_volume_wheel: u8,
    #[bits(452..=459)]
    pub organ_b_volume_aftertouch: u8,
    #[bits(460..=467)]
    pub organ_b_volume_ctrl_pedal: u8,
    #[bits(468..=468)]
    pub organ_pitch_stick_enabled: bool,
    #[bits(472..=472)]
    pub organ_rotary_speaker_enabled: bool,
    #[bits(484..=490)]
    pub rotary_speaker_drive: RangedU8<127>,
    #[bits(508..=510)]
    pub rotary_speaker_stop_position: RangedU8<7>,
    #[bits(514..=514)]
    pub rotary_speaker_stop_enabled: bool,
    #[bits(515..=515)]
    pub rotary_speaker_slow_fast: bool,
    #[bits(522..=528)]
    pub organ_vib_chorus_type: RangedU8<127>,
    #[bits(544..=547)]
    pub organ_a_kb_zones: RangedU8<15>,
    #[bits(548..=551)]
    pub organ_a_octave_shift: RangedU8<15>,
    #[bits(552..=552)]
    pub organ_a_sustain_pedal_enabled: bool,
    #[bits(553..=555)]
    pub organ_a_model: RangedU8<7>,
    #[bits(556..=556)]
    pub organ_a_preset_enabled: bool,
    #[bits(576..=579)]
    pub organ_a_drawbar_1: RangedU8<15>,
    #[bits(580..=584)]
    pub organ_a_drawbar_1_wheel: RangedU8<31>,
    #[bits(585..=589)]
    pub organ_a_drawbar_1_aftertouch: RangedU8<31>,
    #[bits(590..=594)]
    pub organ_a_drawbar_1_ctrl_pedal: RangedU8<31>,
    #[bits(595..=598)]
    pub organ_a_drawbar_2: RangedU8<15>,
    #[bits(599..=603)]
    pub organ_a_drawbar_2_wheel: RangedU8<31>,
    #[bits(604..=608)]
    pub organ_a_drawbar_2_aftertouch: RangedU8<31>,
    #[bits(609..=613)]
    pub organ_a_drawbar_2_ctrl_pedal: RangedU8<31>,
    #[bits(614..=617)]
    pub organ_a_drawbar_3: RangedU8<15>,
    #[bits(618..=622)]
    pub organ_a_drawbar_3_wheel: RangedU8<31>,
    #[bits(623..=627)]
    pub organ_a_drawbar_3_aftertouch: RangedU8<31>,
    #[bits(628..=632)]
    pub organ_a_drawbar_3_ctrl_pedal: RangedU8<31>,
    #[bits(633..=636)]
    pub organ_a_drawbar_4: RangedU8<15>,
    #[bits(637..=641)]
    pub organ_a_drawbar_4_wheel: RangedU8<31>,
    #[bits(642..=646)]
    pub organ_a_drawbar_4_aftertouch: RangedU8<31>,
    #[bits(647..=651)]
    pub organ_a_drawbar_4_ctrl_pedal: RangedU8<31>,
    #[bits(652..=655)]
    pub organ_a_drawbar_5: RangedU8<15>,
    #[bits(656..=660)]
    pub organ_a_drawbar_5_wheel: RangedU8<31>,
    #[bits(661..=665)]
    pub organ_a_drawbar_5_aftertouch: RangedU8<31>,
    #[bits(666..=670)]
    pub organ_a_drawbar_5_ctrl_pedal: RangedU8<31>,
    #[bits(671..=674)]
    pub organ_a_drawbar_6: RangedU8<15>,
    #[bits(675..=679)]
    pub organ_a_drawbar_6_wheel: RangedU8<31>,
    #[bits(680..=684)]
    pub organ_a_drawbar_6_aftertouch: RangedU8<31>,
    #[bits(685..=689)]
    pub organ_a_drawbar_6_ctrl_pedal: RangedU8<31>,
    #[bits(690..=693)]
    pub organ_a_drawbar_7: RangedU8<15>,
    #[bits(694..=698)]
    pub organ_a_drawbar_7_wheel: RangedU8<31>,
    #[bits(699..=703)]
    pub organ_a_drawbar_7_aftertouch: RangedU8<31>,
    #[bits(704..=708)]
    pub organ_a_drawbar_7_ctrl_pedal: RangedU8<31>,
    #[bits(709..=712)]
    pub organ_a_drawbar_8: RangedU8<15>,
    #[bits(713..=717)]
    pub organ_a_drawbar_8_wheel: RangedU8<31>,
    #[bits(718..=722)]
    pub organ_a_drawbar_8_aftertouch: RangedU8<31>,
    #[bits(723..=727)]
    pub organ_a_drawbar_8_ctrl_pedal: RangedU8<31>,
    #[bits(728..=731)]
    pub organ_a_drawbar_9: RangedU8<15>,
    #[bits(732..=736)]
    pub organ_a_drawbar_9_wheel: RangedU8<31>,
    #[bits(737..=741)]
    pub organ_a_drawbar_9_aftertouch: RangedU8<31>,
    #[bits(742..=746)]
    pub organ_a_drawbar_9_ctrl_pedal: RangedU8<31>,
    #[bits(768..=768)]
    pub organ_a_vib_chorus_enabled: bool,
    #[bits(769..=769)]
    pub organ_a_percussion_enabled: bool,
    #[bits(770..=770)]
    pub organ_a_percussion_harmonic_3rd_enabled: bool,
    #[bits(771..=771)]
    pub organ_a_percussion_decay_fast_enabled: bool,
    #[bits(772..=772)]
    pub organ_a_percussion_volume_soft_enabled: bool,
    #[bits(792..=795)]
    pub organ_b_kb_zones: RangedU8<15>,
    #[bits(796..=799)]
    pub organ_b_octave_shift: RangedU8<15>,
    #[bits(800..=800)]
    pub organ_b_sustain_pedal_enabled: bool,
    #[bits(801..=803)]
    pub organ_b_model: RangedU8<7>,
    #[bits(804..=804)]
    pub organ_b_preset_enabled: bool,
    #[bits(824..=827)]
    pub organ_b_drawbar_1: RangedU8<15>,
    #[bits(828..=832)]
    pub organ_b_drawbar_1_wheel: RangedU8<31>,
    #[bits(833..=837)]
    pub organ_b_drawbar_1_aftertouch: RangedU8<31>,
    #[bits(838..=842)]
    pub organ_b_drawbar_1_ctrl_pedal: RangedU8<31>,
    #[bits(843..=846)]
    pub organ_b_drawbar_2: RangedU8<15>,
    #[bits(847..=851)]
    pub organ_b_drawbar_2_wheel: RangedU8<31>,
    #[bits(852..=856)]
    pub organ_b_drawbar_2_aftertouch: RangedU8<31>,
    #[bits(857..=861)]
    pub organ_b_drawbar_2_ctrl_pedal: RangedU8<31>,
    #[bits(862..=865)]
    pub organ_b_drawbar_3: RangedU8<15>,
    #[bits(866..=870)]
    pub organ_b_drawbar_3_wheel: RangedU8<31>,
    #[bits(871..=875)]
    pub organ_b_drawbar_3_aftertouch: RangedU8<31>,
    #[bits(876..=880)]
    pub organ_b_drawbar_3_ctrl_pedal: RangedU8<31>,
    #[bits(881..=884)]
    pub organ_b_drawbar_4: RangedU8<15>,
    #[bits(885..=889)]
    pub organ_b_drawbar_4_wheel: RangedU8<31>,
    #[bits(890..=894)]
    pub organ_b_drawbar_4_aftertouch: RangedU8<31>,
    #[bits(895..=899)]
    pub organ_b_drawbar_4_ctrl_pedal: RangedU8<31>,
    #[bits(900..=903)]
    pub organ_b_drawbar_5: RangedU8<15>,
    #[bits(904..=908)]
    pub organ_b_drawbar_5_wheel: RangedU8<31>,
    #[bits(909..=913)]
    pub organ_b_drawbar_5_aftertouch: RangedU8<31>,
    #[bits(914..=918)]
    pub organ_b_drawbar_5_ctrl_pedal: RangedU8<31>,
    #[bits(919..=922)]
    pub organ_b_drawbar_6: RangedU8<15>,
    #[bits(923..=927)]
    pub organ_b_drawbar_6_wheel: RangedU8<31>,
    #[bits(928..=932)]
    pub organ_b_drawbar_6_aftertouch: RangedU8<31>,
    #[bits(933..=937)]
    pub organ_b_drawbar_6_ctrl_pedal: RangedU8<31>,
    #[bits(938..=941)]
    pub organ_b_drawbar_7: RangedU8<15>,
    #[bits(942..=946)]
    pub organ_b_drawbar_7_wheel: RangedU8<31>,
    #[bits(947..=951)]
    pub organ_b_drawbar_7_aftertouch: RangedU8<31>,
    #[bits(952..=956)]
    pub organ_b_drawbar_7_ctrl_pedal: RangedU8<31>,
    #[bits(957..=960)]
    pub organ_b_drawbar_8: RangedU8<15>,
    #[bits(961..=965)]
    pub organ_b_drawbar_8_wheel: RangedU8<31>,
    #[bits(966..=970)]
    pub organ_b_drawbar_8_aftertouch: RangedU8<31>,
    #[bits(971..=975)]
    pub organ_b_drawbar_8_ctrl_pedal: RangedU8<31>,
    #[bits(976..=979)]
    pub organ_b_drawbar_9: RangedU8<15>,
    #[bits(980..=984)]
    pub organ_b_drawbar_9_wheel: RangedU8<31>,
    #[bits(985..=989)]
    pub organ_b_drawbar_9_aftertouch: RangedU8<31>,
    #[bits(990..=994)]
    pub organ_b_drawbar_9_ctrl_pedal: RangedU8<31>,
    #[bits(1016..=1016)]
    pub organ_b_vib_chorus_enabled: bool,
    #[bits(1017..=1017)]
    pub organ_b_percussion_enabled: bool,
    #[bits(1018..=1018)]
    pub organ_b_percussion_harmonic_3rd_enabled: bool,
    #[bits(1019..=1019)]
    pub organ_b_percussion_decay_fast_enabled: bool,
    #[bits(1020..=1020)]
    pub organ_b_percussion_volume_soft_enabled: bool,
    #[bits(1040..=1040)]
    pub organ_fx_mod_1_enabled: bool,
    #[bits(1041..=1041)]
    pub organ_fx_mod_1_master_clock_enabled: bool,
    #[bits(1042..=1048)]
    pub organ_fx_mod_1_rate: RangedU8<127>,
    #[bits(1049..=1056)]
    pub organ_fx_mod_1_rate_wheel: u8,
    #[bits(1057..=1064)]
    pub organ_fx_mod_1_rate_aftertouch: u8,
    #[bits(1065..=1072)]
    pub organ_fx_mod_1_rate_ctrl_pedal: u8,
    #[bits(1073..=1079)]
    pub organ_fx_mod_1_amount: RangedU8<127>,
    #[bits(1080..=1087)]
    pub organ_fx_mod_1_amount_wheel: u8,
    #[bits(1088..=1095)]
    pub organ_fx_mod_1_amount_aftertouch: u8,
    #[bits(1096..=1103)]
    pub organ_fx_mod_1_amount_ctrl_pedal: u8,
    #[bits(1104..=1107)]
    pub organ_fx_mod_1_mode: RangedU8<15>,
    #[bits(1108..=1108)]
    pub organ_fx_mod_2_enabled: bool,
    #[bits(1109..=1115)]
    pub organ_fx_mod_2_rate: RangedU8<127>,
    #[bits(1116..=1123)]
    pub organ_fx_mod_2_rate_wheel: u8,
    #[bits(1124..=1131)]
    pub organ_fx_mod_2_rate_aftertouch: u8,
    #[bits(1132..=1139)]
    pub organ_fx_mod_2_rate_ctrl_pedal: u8,
    #[bits(1140..=1146)]
    pub organ_fx_mod_2_amount: RangedU8<127>,
    #[bits(1147..=1154)]
    pub organ_fx_mod_2_amount_wheel: u8,
    #[bits(1155..=1162)]
    pub organ_fx_mod_2_amount_aftertouch: u8,
    #[bits(1163..=1170)]
    pub organ_fx_mod_2_amount_ctrl_pedal: u8,
    #[bits(1171..=1174)]
    pub organ_fx_mod_2_mode: RangedU8<15>,
    #[bits(1175..=1175)]
    pub organ_fx_amp_sim_eq_enabled: bool,
    #[bits(1176..=1182)]
    pub organ_fx_amp_sim_eq_treb: RangedU8<127>,
    #[bits(1183..=1189)]
    pub organ_fx_amp_sim_eq_mid: RangedU8<127>,
    #[bits(1190..=1196)]
    pub organ_fx_amp_sim_eq_bass: RangedU8<127>,
    #[bits(1197..=1203)]
    pub organ_fx_amp_sim_eq_freq: RangedU8<127>,
    #[bits(1204..=1211)]
    pub organ_fx_amp_sim_eq_freq_wheel: u8,
    #[bits(1212..=1219)]
    pub organ_fx_amp_sim_eq_freq_aftertouch: u8,
    #[bits(1220..=1227)]
    pub organ_fx_amp_sim_eq_freq_ctrl_pedal: u8,
    #[bits(1228..=1234)]
    pub organ_fx_amp_sim_eq_drive: RangedU8<127>,
    #[bits(1235..=1242)]
    pub organ_fx_amp_sim_eq_drive_wheel: u8,
    #[bits(1243..=1250)]
    pub organ_fx_amp_sim_eq_drive_aftertouch: u8,
    #[bits(1251..=1258)]
    pub organ_fx_amp_sim_eq_drive_ctrl_pedal: u8,
    #[bits(1263..=1263)]
    pub organ_fx_comp_enabled: bool,
    #[bits(1264..=1270)]
    pub organ_fx_comp_amount: RangedU8<127>,
    #[bits(1271..=1271)]
    pub organ_fx_comp_response: bool,
    #[bits(1272..=1272)]
    pub organ_fx_delay_enabled: bool,
    #[bits(1273..=1273)]
    pub organ_fx_delay_tempo_master_clock_enabled: bool,
    #[bits(1274..=1280)]
    pub organ_fx_delay_tempo: RangedU8<127>,
    #[bits(1288..=1295)]
    pub organ_fx_delay_tempo_wheel: u8,
    #[bits(1296..=1303)]
    pub organ_fx_delay_tempo_aftertouch: u8,
    #[bits(1304..=1311)]
    pub organ_fx_delay_tempo_ctrl_pedal: u8,
    #[bits(1333..=1339)]
    pub organ_fx_delay_mix: RangedU8<127>,
    #[bits(1340..=1347)]
    pub organ_fx_delay_mix_wheel: u8,
    #[bits(1348..=1355)]
    pub organ_fx_delay_mix_aftertouch: u8,
    #[bits(1356..=1363)]
    pub organ_fx_delay_mix_ctrl_pedal: u8,
    #[bits(1364..=1364)]
    pub organ_fx_delay_normal_analog: bool,
    #[bits(1365..=1365)]
    pub organ_fx_delay_ping_pong_enabled: bool,
    #[bits(1366..=1367)]
    pub organ_fx_delay_filter_type: RangedU8<3>,
    #[bits(1368..=1374)]
    pub organ_fx_delay_feedback: RangedU8<127>,
    #[bits(1375..=1382)]
    pub organ_fx_delay_feedback_wheel: u8,
    #[bits(1383..=1390)]
    pub organ_fx_delay_feedback_aftertouch: u8,
    #[bits(1391..=1398)]
    pub organ_fx_delay_feedback_ctrl_pedal: u8,
    #[bits(1399..=1402)]
    pub organ_fx_delay_effects: RangedU8<15>,
    #[bits(1403..=1403)]
    pub organ_fx_reverb_enabled: bool,
    #[bits(1404..=1410)]
    pub organ_fx_reverb_amount: RangedU8<127>,
    #[bits(1411..=1418)]
    pub organ_fx_reverb_amount_wheel: u8,
    #[bits(1419..=1426)]
    pub organ_fx_reverb_amount_aftertouch: u8,
    #[bits(1427..=1434)]
    pub organ_fx_reverb_amount_ctrl_pedal: u8,
    #[bits(1435..=1436)]
    pub organ_fx_reverb_dark_bright: RangedU8<3>,
    #[bits(1437..=1440)]
    pub organ_fx_reverb_type: RangedU8<15>,
    #[bits(1445..=1448)]
    pub organ_fx_amp_sim_eq_mode: RangedU8<15>,
    #[bits(1481..=1481)]
    pub piano_b_layer_enabled: bool,
    #[bits(1482..=1482)]
    pub piano_a_layer_enabled: bool,
    #[bits(1484..=1484)]
    pub piano_b_layer_enabled_scene_2: bool,
    #[bits(1485..=1485)]
    pub piano_a_layer_enabled_scene_2: bool,
    #[bits(1486..=1492)]
    pub piano_a_volume: RangedU8<127>,
    #[bits(1493..=1500)]
    pub piano_a_volume_wheel: u8,
    #[bits(1501..=1508)]
    pub piano_a_volume_aftertouch: u8,
    #[bits(1509..=1516)]
    pub piano_a_volume_ctrl_pedal: u8,
    #[bits(1517..=1523)]
    pub piano_b_volume: RangedU8<127>,
    #[bits(1524..=1531)]
    pub piano_b_volume_wheel: u8,
    #[bits(1532..=1539)]
    pub piano_b_volume_aftertouch: u8,
    #[bits(1540..=1547)]
    pub piano_b_volume_ctrl_pedal: u8,
    #[bits(1584..=1587)]
    pub piano_a_kb_zones: RangedU8<15>,
    #[bits(1588..=1591)]
    pub piano_a_octave_shift: RangedU8<15>,
    #[bits(1592..=1592)]
    pub piano_a_pitch_stick_enabled: bool,
    #[bits(1593..=1593)]
    pub piano_a_sustain_pedal_enabled: bool,
    #[bits(1594..=1596)]
    pub piano_a_type: RangedU8<7>,
    #[bits(1597..=1601)]
    pub piano_a_model_slot: RangedU8<31>,
    #[bits(1602..=1603)]
    pub piano_a_model_variation: RangedU8<3>,
    #[bits(1604..=1635)]
    pub piano_a_model_id: u32,
    #[bits(1636..=1636)]
    pub piano_a_soft_rel_enabled: bool,
    #[bits(1637..=1637)]
    pub piano_a_string_res_enabled: bool,
    #[bits(1638..=1638)]
    pub piano_a_pedal_noise_enabled: bool,
    #[bits(1639..=1640)]
    pub piano_a_touch: RangedU8<3>,
    #[bits(1641..=1642)]
    pub piano_a_unison_level: RangedU8<3>,
    #[bits(1643..=1644)]
    pub piano_a_dyn_comp: RangedU8<3>,
    #[bits(1646..=1648)]
    pub piano_a_timbre: RangedU8<7>,
    #[bits(1680..=1683)]
    pub piano_b_kb_zones: RangedU8<15>,
    #[bits(1684..=1687)]
    pub piano_b_octave_shift: RangedU8<15>,
    #[bits(1688..=1688)]
    pub piano_b_pitch_stick_enabled: bool,
    #[bits(1689..=1689)]
    pub piano_b_sustain_pedal_enabled: bool,
    #[bits(1690..=1692)]
    pub piano_b_type: RangedU8<7>,
    #[bits(1693..=1697)]
    pub piano_b_model_slot: RangedU8<31>,
    #[bits(1698..=1699)]
    pub piano_b_model_variation: RangedU8<3>,
    #[bits(1700..=1731)]
    pub piano_b_model_id: u32,
    #[bits(1732..=1732)]
    pub piano_b_soft_rel_enabled: bool,
    #[bits(1733..=1733)]
    pub piano_b_string_res_enabled: bool,
    #[bits(1734..=1734)]
    pub piano_b_pedal_noise_enabled: bool,
    #[bits(1735..=1736)]
    pub piano_b_touch: RangedU8<3>,
    #[bits(1737..=1738)]
    pub piano_b_unison_level: RangedU8<3>,
    #[bits(1739..=1740)]
    pub piano_b_dyn_comp: RangedU8<3>,
    #[bits(1742..=1744)]
    pub piano_b_timbre: RangedU8<7>,
    #[bits(1776..=1776)]
    pub piano_a_fx_mod_1_enabled: bool,
    #[bits(1777..=1777)]
    pub piano_a_fx_mod_1_master_clock_enabled: bool,
    #[bits(1778..=1784)]
    pub piano_a_fx_mod_1_rate: RangedU8<127>,
    #[bits(1785..=1792)]
    pub piano_a_fx_mod_1_rate_wheel: u8,
    #[bits(1793..=1800)]
    pub piano_a_fx_mod_1_rate_aftertouch: u8,
    #[bits(1801..=1808)]
    pub piano_a_fx_mod_1_rate_ctrl_pedal: u8,
    #[bits(1809..=1815)]
    pub piano_a_fx_mod_1_amount: RangedU8<127>,
    #[bits(1816..=1823)]
    pub piano_a_fx_mod_1_amount_wheel: u8,
    #[bits(1824..=1831)]
    pub piano_a_fx_mod_1_amount_aftertouch: u8,
    #[bits(1832..=1839)]
    pub piano_a_fx_mod_1_amount_ctrl_pedal: u8,
    #[bits(1840..=1843)]
    pub piano_a_fx_mod_1_mode: RangedU8<15>,
    #[bits(1844..=1844)]
    pub piano_a_fx_mod_2_enabled: bool,
    #[bits(1845..=1851)]
    pub piano_a_fx_mod_2_rate: RangedU8<127>,
    #[bits(1852..=1859)]
    pub piano_a_fx_mod_2_rate_wheel: u8,
    #[bits(1860..=1867)]
    pub piano_a_fx_mod_2_rate_aftertouch: u8,
    #[bits(1868..=1875)]
    pub piano_a_fx_mod_2_rate_ctrl_pedal: u8,
    #[bits(1876..=1882)]
    pub piano_a_fx_mod_2_amount: RangedU8<127>,
    #[bits(1883..=1890)]
    pub piano_a_fx_mod_2_amount_wheel: u8,
    #[bits(1891..=1898)]
    pub piano_a_fx_mod_2_amount_aftertouch: u8,
    #[bits(1899..=1906)]
    pub piano_a_fx_mod_2_amount_ctrl_pedal: u8,
    #[bits(1907..=1910)]
    pub piano_a_fx_mod_2_mode: RangedU8<15>,
    #[bits(1911..=1911)]
    pub piano_a_fx_amp_sim_eq_enabled: bool,
    #[bits(1912..=1918)]
    pub piano_a_fx_amp_sim_eq_treb: RangedU8<127>,
    #[bits(1919..=1925)]
    pub piano_a_fx_amp_sim_eq_mid: RangedU8<127>,
    #[bits(1926..=1932)]
    pub piano_a_fx_amp_sim_eq_bass: RangedU8<127>,
    #[bits(1933..=1939)]
    pub piano_a_fx_amp_sim_eq_freq: RangedU8<127>,
    #[bits(1940..=1947)]
    pub piano_a_fx_amp_sim_eq_freq_wheel: u8,
    #[bits(1948..=1955)]
    pub piano_a_fx_amp_sim_eq_freq_aftertouch: u8,
    #[bits(1956..=1963)]
    pub piano_a_fx_amp_sim_eq_freq_ctrl_pedal: u8,
    #[bits(1964..=1970)]
    pub piano_a_fx_amp_sim_eq_drive: RangedU8<127>,
    #[bits(1971..=1978)]
    pub piano_a_fx_amp_sim_eq_drive_wheel: u8,
    #[bits(1979..=1986)]
    pub piano_a_fx_amp_sim_eq_drive_aftertouch: u8,
    #[bits(1987..=1994)]
    pub piano_a_fx_amp_sim_eq_drive_ctrl_pedal: u8,
    #[bits(1999..=1999)]
    pub piano_a_fx_comp_enabled: bool,
    #[bits(2000..=2006)]
    pub piano_a_fx_comp_amount: RangedU8<127>,
    #[bits(2007..=2007)]
    pub piano_a_fx_comp_response: bool,
    #[bits(2008..=2008)]
    pub piano_a_fx_delay_enabled: bool,
    #[bits(2009..=2009)]
    pub piano_a_fx_delay_tempo_master_clock_enabled: bool,
    #[bits(2010..=2016)]
    pub piano_a_fx_delay_tempo: RangedU8<127>,
    #[bits(2024..=2031)]
    pub piano_a_fx_delay_tempo_wheel: u8,
    #[bits(2032..=2039)]
    pub piano_a_fx_delay_tempo_aftertouch: u8,
    #[bits(2040..=2047)]
    pub piano_a_fx_delay_tempo_ctrl_pedal: u8,
    #[bits(2069..=2075)]
    pub piano_a_fx_delay_mix: RangedU8<127>,
    #[bits(2076..=2083)]
    pub piano_a_fx_delay_mix_wheel: u8,
    #[bits(2084..=2091)]
    pub piano_a_fx_delay_mix_aftertouch: u8,
    #[bits(2092..=2099)]
    pub piano_a_fx_delay_mix_ctrl_pedal: u8,
    #[bits(2100..=2100)]
    pub piano_a_fx_delay_normal_analog: bool,
    #[bits(2101..=2101)]
    pub piano_a_fx_delay_ping_pong_enabled: bool,
    #[bits(2102..=2103)]
    pub piano_a_fx_delay_filter_type: RangedU8<3>,
    #[bits(2104..=2110)]
    pub piano_a_fx_delay_feedback: RangedU8<127>,
    #[bits(2111..=2118)]
    pub piano_a_fx_delay_feedback_wheel: u8,
    #[bits(2119..=2126)]
    pub piano_a_fx_delay_feedback_aftertouch: u8,
    #[bits(2127..=2134)]
    pub piano_a_fx_delay_feedback_ctrl_pedal: u8,
    #[bits(2135..=2138)]
    pub piano_a_fx_delay_effects: RangedU8<15>,
    #[bits(2139..=2139)]
    pub piano_a_fx_reverb_enabled: bool,
    #[bits(2140..=2146)]
    pub piano_a_fx_reverb_amount: RangedU8<127>,
    #[bits(2147..=2154)]
    pub piano_a_fx_reverb_amount_wheel: u8,
    #[bits(2155..=2162)]
    pub piano_a_fx_reverb_amount_aftertouch: u8,
    #[bits(2163..=2170)]
    pub piano_a_fx_reverb_amount_ctrl_pedal: u8,
    #[bits(2171..=2172)]
    pub piano_a_fx_reverb_dark_bright: RangedU8<3>,
    #[bits(2173..=2176)]
    pub piano_a_fx_reverb_type: RangedU8<15>,
    #[bits(2181..=2184)]
    pub piano_a_fx_amp_sim_eq_mode: RangedU8<15>,
    #[bits(2216..=2216)]
    pub piano_b_fx_mod_1_enabled: bool,
    #[bits(2217..=2217)]
    pub piano_b_fx_mod_1_master_clock_enabled: bool,
    #[bits(2218..=2224)]
    pub piano_b_fx_mod_1_rate: RangedU8<127>,
    #[bits(2225..=2232)]
    pub piano_b_fx_mod_1_rate_wheel: u8,
    #[bits(2233..=2240)]
    pub piano_b_fx_mod_1_rate_aftertouch: u8,
    #[bits(2241..=2248)]
    pub piano_b_fx_mod_1_rate_ctrl_pedal: u8,
    #[bits(2249..=2255)]
    pub piano_b_fx_mod_1_amount: RangedU8<127>,
    #[bits(2256..=2263)]
    pub piano_b_fx_mod_1_amount_wheel: u8,
    #[bits(2264..=2271)]
    pub piano_b_fx_mod_1_amount_aftertouch: u8,
    #[bits(2272..=2279)]
    pub piano_b_fx_mod_1_amount_ctrl_pedal: u8,
    #[bits(2280..=2283)]
    pub piano_b_fx_mod_1_mode: RangedU8<15>,
    #[bits(2284..=2284)]
    pub piano_b_fx_mod_2_enabled: bool,
    #[bits(2285..=2291)]
    pub piano_b_fx_mod_2_rate: RangedU8<127>,
    #[bits(2292..=2299)]
    pub piano_b_fx_mod_2_rate_wheel: u8,
    #[bits(2300..=2307)]
    pub piano_b_fx_mod_2_rate_aftertouch: u8,
    #[bits(2308..=2315)]
    pub piano_b_fx_mod_2_rate_ctrl_pedal: u8,
    #[bits(2316..=2322)]
    pub piano_b_fx_mod_2_amount: RangedU8<127>,
    #[bits(2323..=2330)]
    pub piano_b_fx_mod_2_amount_wheel: u8,
    #[bits(2331..=2338)]
    pub piano_b_fx_mod_2_amount_aftertouch: u8,
    #[bits(2339..=2346)]
    pub piano_b_fx_mod_2_amount_ctrl_pedal: u8,
    #[bits(2347..=2350)]
    pub piano_b_fx_mod_2_mode: RangedU8<15>,
    #[bits(2351..=2351)]
    pub piano_b_fx_amp_sim_eq_enabled: bool,
    #[bits(2352..=2358)]
    pub piano_b_fx_amp_sim_eq_treb: RangedU8<127>,
    #[bits(2359..=2365)]
    pub piano_b_fx_amp_sim_eq_mid: RangedU8<127>,
    #[bits(2366..=2372)]
    pub piano_b_fx_amp_sim_eq_bass: RangedU8<127>,
    #[bits(2373..=2379)]
    pub piano_b_fx_amp_sim_eq_freq: RangedU8<127>,
    #[bits(2380..=2387)]
    pub piano_b_fx_amp_sim_eq_freq_wheel: u8,
    #[bits(2388..=2395)]
    pub piano_b_fx_amp_sim_eq_freq_aftertouch: u8,
    #[bits(2396..=2403)]
    pub piano_b_fx_amp_sim_eq_freq_ctrl_pedal: u8,
    #[bits(2404..=2410)]
    pub piano_b_fx_amp_sim_eq_drive: RangedU8<127>,
    #[bits(2411..=2418)]
    pub piano_b_fx_amp_sim_eq_drive_wheel: u8,
    #[bits(2419..=2426)]
    pub piano_b_fx_amp_sim_eq_drive_aftertouch: u8,
    #[bits(2427..=2434)]
    pub piano_b_fx_amp_sim_eq_drive_ctrl_pedal: u8,
    #[bits(2439..=2439)]
    pub piano_b_fx_comp_enabled: bool,
    #[bits(2440..=2446)]
    pub piano_b_fx_comp_amount: RangedU8<127>,
    #[bits(2447..=2447)]
    pub piano_b_fx_comp_response: bool,
    #[bits(2448..=2448)]
    pub piano_b_fx_delay_enabled: bool,
    #[bits(2449..=2449)]
    pub piano_b_fx_delay_tempo_master_clock_enabled: bool,
    #[bits(2450..=2456)]
    pub piano_b_fx_delay_tempo: RangedU8<127>,
    #[bits(2464..=2471)]
    pub piano_b_fx_delay_tempo_wheel: u8,
    #[bits(2472..=2479)]
    pub piano_b_fx_delay_tempo_aftertouch: u8,
    #[bits(2480..=2487)]
    pub piano_b_fx_delay_tempo_ctrl_pedal: u8,
    #[bits(2509..=2515)]
    pub piano_b_fx_delay_mix: RangedU8<127>,
    #[bits(2516..=2523)]
    pub piano_b_fx_delay_mix_wheel: u8,
    #[bits(2524..=2531)]
    pub piano_b_fx_delay_mix_aftertouch: u8,
    #[bits(2532..=2539)]
    pub piano_b_fx_delay_mix_ctrl_pedal: u8,
    #[bits(2540..=2540)]
    pub piano_b_fx_delay_normal_analog: bool,
    #[bits(2541..=2541)]
    pub piano_b_fx_delay_ping_pong_enabled: bool,
    #[bits(2542..=2543)]
    pub piano_b_fx_delay_filter_type: RangedU8<3>,
    #[bits(2544..=2550)]
    pub piano_b_fx_delay_feedback: RangedU8<127>,
    #[bits(2551..=2558)]
    pub piano_b_fx_delay_feedback_wheel: u8,
    #[bits(2559..=2566)]
    pub piano_b_fx_delay_feedback_aftertouch: u8,
    #[bits(2567..=2574)]
    pub piano_b_fx_delay_feedback_ctrl_pedal: u8,
    #[bits(2575..=2578)]
    pub piano_b_fx_delay_effects: RangedU8<15>,
    #[bits(2579..=2579)]
    pub piano_b_fx_reverb_enabled: bool,
    #[bits(2580..=2586)]
    pub piano_b_fx_reverb_amount: RangedU8<127>,
    #[bits(2587..=2594)]
    pub piano_b_fx_reverb_amount_wheel: u8,
    #[bits(2595..=2602)]
    pub piano_b_fx_reverb_amount_aftertouch: u8,
    #[bits(2603..=2610)]
    pub piano_b_fx_reverb_amount_ctrl_pedal: u8,
    #[bits(2611..=2612)]
    pub piano_b_fx_reverb_dark_bright: RangedU8<3>,
    #[bits(2613..=2616)]
    pub piano_b_fx_reverb_type: RangedU8<15>,
    #[bits(2621..=2624)]
    pub piano_b_fx_amp_sim_eq_mode: RangedU8<15>,
    #[bits(2658..=2658)]
    pub synth_c_layer_enabled: bool,
    #[bits(2659..=2659)]
    pub synth_b_layer_enabled: bool,
    #[bits(2660..=2660)]
    pub synth_a_layer_enabled: bool,
    #[bits(2663..=2663)]
    pub synth_c_layer_enabled_scene_2: bool,
    #[bits(2664..=2664)]
    pub synth_b_layer_enabled_scene_2: bool,
    #[bits(2665..=2665)]
    pub synth_a_layer_enabled_scene_2: bool,
    #[bits(2666..=2672)]
    pub synth_a_volume: RangedU8<127>,
    #[bits(2673..=2680)]
    pub synth_a_volume_wheel: u8,
    #[bits(2681..=2688)]
    pub synth_a_volume_aftertouch: u8,
    #[bits(2689..=2696)]
    pub synth_a_volume_ctrl_pedal: u8,
    #[bits(2697..=2703)]
    pub synth_b_volume: RangedU8<127>,
    #[bits(2704..=2711)]
    pub synth_b_volume_wheel: u8,
    #[bits(2712..=2719)]
    pub synth_b_volume_aftertouch: u8,
    #[bits(2720..=2727)]
    pub synth_b_volume_ctrl_pedal: u8,
    #[bits(2728..=2734)]
    pub synth_c_volume: RangedU8<127>,
    #[bits(2735..=2742)]
    pub synth_c_volume_wheel: u8,
    #[bits(2743..=2750)]
    pub synth_c_volume_aftertouch: u8,
    #[bits(2751..=2758)]
    pub synth_c_volume_ctrl_pedal: u8,
    #[bits(2759..=2764)]
    pub synth_a_pan: RangedU8<63>,
    #[bits(2790..=2795)]
    pub synth_b_pan: RangedU8<63>,
    #[bits(2821..=2826)]
    pub synth_c_pan: RangedU8<63>,
    #[bits(2853..=2853)]
    pub synth_arp_group_enabled: bool,
    #[bits(2855..=2855)]
    pub synth_kb_hold_enabled: bool,
    #[bits(2904..=2904)]
    pub synth_a_samples_analog: bool,
    #[bits(2910..=2921)]
    pub synth_a_sample_slot: RangedU16<4095>,
    #[bits(2922..=2953)]
    pub synth_a_sample_id: u32,
    #[bits(2954..=2957)]
    pub synth_a_kb_zones: RangedU8<15>,
    #[bits(2958..=2961)]
    pub synth_a_octave_shift: RangedU8<15>,
    #[bits(2962..=2962)]
    pub synth_a_pitch_stick_enabled: bool,
    #[bits(2963..=2966)]
    pub synth_a_pitch_stick_range: RangedU8<15>,
    #[bits(2967..=2967)]
    pub synth_a_sustain_pedal_enabled: bool,
    #[bits(2968..=2970)]
    pub synth_a_vibrato_mode: RangedU8<7>,
    #[bits(2973..=2973)]
    pub synth_a_legato_enabled: bool,
    #[bits(2974..=2974)]
    pub synth_a_mono_enabled: bool,
    #[bits(2975..=2976)]
    pub synth_a_voice_priority: RangedU8<3>,
    #[bits(2977..=2983)]
    pub synth_a_glide: RangedU8<127>,
    #[bits(2984..=2984)]
    pub synth_a_extern_enabled: bool,
    #[bits(3015..=3021)]
    pub synth_a_extern_program: RangedU8<127>,
    #[bits(3030..=3030)]
    pub synth_a_kb_hold: bool,
    #[bits(3031..=3031)]
    pub synth_a_arpeggiator_run_enabled: bool,
    #[bits(3032..=3033)]
    pub synth_a_arpeggiator_mode: RangedU8<3>,
    #[bits(3034..=3034)]
    pub synth_a_arp_pattern_enabled: bool,
    #[bits(3035..=3035)]
    pub synth_a_kb_sync_enabled: bool,
    #[bits(3036..=3042)]
    pub synth_a_arp_range_env: RangedU8<127>,
    #[bits(3043..=3050)]
    pub synth_a_arp_range_env_wheel: u8,
    #[bits(3051..=3058)]
    pub synth_a_arp_range_env_aftertouch: u8,
    #[bits(3059..=3066)]
    pub synth_a_arp_range_env_ctrl_pedal: u8,
    #[bits(3067..=3068)]
    pub synth_a_arp_direction: RangedU8<3>,
    #[bits(3069..=3069)]
    pub synth_a_arp_zigzag_enabled: bool,
    #[bits(3070..=3070)]
    pub synth_a_arp_master_clock_enabled: bool,
    #[bits(3071..=3077)]
    pub synth_a_arp_rate_time: RangedU8<127>,
    #[bits(3078..=3085)]
    pub synth_a_arp_rate_time_wheel: u8,
    #[bits(3086..=3093)]
    pub synth_a_arp_rate_time_aftertouch: u8,
    #[bits(3094..=3101)]
    pub synth_a_arp_rate_time_ctrl_pedal: u8,
    #[bits(3102..=3105)]
    pub synth_a_arp_pattern_length: RangedU8<15>,
    #[bits(3106..=3137)]
    pub synth_a_arpeggiator_accent: u32,
    #[bits(3138..=3169)]
    pub synth_a_arpeggiator_gate: u32,
    #[bits(3170..=3201)]
    pub synth_a_arpeggiator_pan: u32,
    #[bits(3202..=3203)]
    pub synth_a_unison_level: RangedU8<3>,
    #[bits(3204..=3210)]
    pub synth_a_extern_cc_val1: RangedU8<127>,
    #[bits(3211..=3218)]
    pub synth_a_extern_cc_val1_wheel: u8,
    #[bits(3219..=3226)]
    pub synth_a_extern_cc_val1_aftertouch: u8,
    #[bits(3227..=3234)]
    pub synth_a_extern_cc_val1_ctrl_pedal: u8,
    #[bits(3235..=3241)]
    pub synth_a_extern_cc_val2: RangedU8<127>,
    #[bits(3242..=3249)]
    pub synth_a_extern_cc_val2_wheel: u8,
    #[bits(3250..=3257)]
    pub synth_a_extern_cc_val2_aftertouch: u8,
    #[bits(3258..=3265)]
    pub synth_a_extern_cc_val2_ctrl_pedal: u8,
    #[bits(3268..=3272)]
    pub synth_a_vibrato_delay: RangedU8<31>,
    #[bits(3312..=3312)]
    pub synth_b_samples_analog: bool,
    #[bits(3318..=3329)]
    pub synth_b_sample_slot: RangedU16<4095>,
    #[bits(3330..=3361)]
    pub synth_b_sample_id: u32,
    #[bits(3362..=3365)]
    pub synth_b_kb_zones: RangedU8<15>,
    #[bits(3366..=3369)]
    pub synth_b_octave_shift: RangedU8<15>,
    #[bits(3370..=3370)]
    pub synth_b_pitch_stick_enabled: bool,
    #[bits(3371..=3374)]
    pub synth_b_pitch_stick_range: RangedU8<15>,
    #[bits(3375..=3375)]
    pub synth_b_sustain_pedal_enabled: bool,
    #[bits(3376..=3378)]
    pub synth_b_vibrato_mode: RangedU8<7>,
    #[bits(3381..=3381)]
    pub synth_b_legato_enabled: bool,
    #[bits(3382..=3382)]
    pub synth_b_mono_enabled: bool,
    #[bits(3383..=3384)]
    pub synth_b_voice_priority: RangedU8<3>,
    #[bits(3385..=3391)]
    pub synth_b_glide: RangedU8<127>,
    #[bits(3392..=3392)]
    pub synth_b_extern_enabled: bool,
    #[bits(3423..=3429)]
    pub synth_b_extern_program: RangedU8<127>,
    #[bits(3438..=3438)]
    pub synth_b_kb_hold: bool,
    #[bits(3439..=3439)]
    pub synth_b_arpeggiator_run_enabled: bool,
    #[bits(3440..=3441)]
    pub synth_b_arpeggiator_mode: RangedU8<3>,
    #[bits(3442..=3442)]
    pub synth_b_arp_pattern_enabled: bool,
    #[bits(3443..=3443)]
    pub synth_b_kb_sync_enabled: bool,
    #[bits(3444..=3450)]
    pub synth_b_arp_range_env: RangedU8<127>,
    #[bits(3451..=3458)]
    pub synth_b_arp_range_env_wheel: u8,
    #[bits(3459..=3466)]
    pub synth_b_arp_range_env_aftertouch: u8,
    #[bits(3467..=3474)]
    pub synth_b_arp_range_env_ctrl_pedal: u8,
    #[bits(3475..=3476)]
    pub synth_b_arp_direction: RangedU8<3>,
    #[bits(3477..=3477)]
    pub synth_b_arp_zigzag_enabled: bool,
    #[bits(3478..=3478)]
    pub synth_b_arp_master_clock_enabled: bool,
    #[bits(3479..=3485)]
    pub synth_b_arp_rate_time: RangedU8<127>,
    #[bits(3486..=3493)]
    pub synth_b_arp_rate_time_wheel: u8,
    #[bits(3494..=3501)]
    pub synth_b_arp_rate_time_aftertouch: u8,
    #[bits(3502..=3509)]
    pub synth_b_arp_rate_time_ctrl_pedal: u8,
    #[bits(3510..=3513)]
    pub synth_b_arp_pattern_length: RangedU8<15>,
    #[bits(3514..=3545)]
    pub synth_b_arpeggiator_accent: u32,
    #[bits(3546..=3577)]
    pub synth_b_arpeggiator_gate: u32,
    #[bits(3578..=3609)]
    pub synth_b_arpeggiator_pan: u32,
    #[bits(3610..=3611)]
    pub synth_b_unison_level: RangedU8<3>,
    #[bits(3612..=3618)]
    pub synth_b_extern_cc_val1: RangedU8<127>,
    #[bits(3619..=3626)]
    pub synth_b_extern_cc_val1_wheel: u8,
    #[bits(3627..=3634)]
    pub synth_b_extern_cc_val1_aftertouch: u8,
    #[bits(3635..=3642)]
    pub synth_b_extern_cc_val1_ctrl_pedal: u8,
    #[bits(3643..=3649)]
    pub synth_b_extern_cc_val2: RangedU8<127>,
    #[bits(3650..=3657)]
    pub synth_b_extern_cc_val2_wheel: u8,
    #[bits(3658..=3665)]
    pub synth_b_extern_cc_val2_aftertouch: u8,
    #[bits(3666..=3673)]
    pub synth_b_extern_cc_val2_ctrl_pedal: u8,
    #[bits(3676..=3680)]
    pub synth_b_vibrato_delay: RangedU8<31>,
    #[bits(3720..=3720)]
    pub synth_c_samples_analog: bool,
    #[bits(3726..=3737)]
    pub synth_c_sample_slot: RangedU16<4095>,
    #[bits(3738..=3769)]
    pub synth_c_sample_id: u32,
    #[bits(3770..=3773)]
    pub synth_c_kb_zones: RangedU8<15>,
    #[bits(3774..=3777)]
    pub synth_c_octave_shift: RangedU8<15>,
    #[bits(3778..=3778)]
    pub synth_c_pitch_stick_enabled: bool,
    #[bits(3779..=3782)]
    pub synth_c_pitch_stick_range: RangedU8<15>,
    #[bits(3783..=3783)]
    pub synth_c_sustain_pedal_enabled: bool,
    #[bits(3784..=3786)]
    pub synth_c_vibrato_mode: RangedU8<7>,
    #[bits(3789..=3789)]
    pub synth_c_legato_enabled: bool,
    #[bits(3790..=3790)]
    pub synth_c_mono_enabled: bool,
    #[bits(3791..=3792)]
    pub synth_c_voice_priority: RangedU8<3>,
    #[bits(3793..=3799)]
    pub synth_c_glide: RangedU8<127>,
    #[bits(3800..=3800)]
    pub synth_c_extern_enabled: bool,
    #[bits(3831..=3837)]
    pub synth_c_extern_program: RangedU8<127>,
    #[bits(3846..=3846)]
    pub synth_c_kb_hold: bool,
    #[bits(3847..=3847)]
    pub synth_c_arpeggiator_run_enabled: bool,
    #[bits(3848..=3849)]
    pub synth_c_arpeggiator_mode: RangedU8<3>,
    #[bits(3850..=3850)]
    pub synth_c_arp_pattern_enabled: bool,
    #[bits(3851..=3851)]
    pub synth_c_kb_sync_enabled: bool,
    #[bits(3852..=3858)]
    pub synth_c_arp_range_env: RangedU8<127>,
    #[bits(3859..=3866)]
    pub synth_c_arp_range_env_wheel: u8,
    #[bits(3867..=3874)]
    pub synth_c_arp_range_env_aftertouch: u8,
    #[bits(3875..=3882)]
    pub synth_c_arp_range_env_ctrl_pedal: u8,
    #[bits(3883..=3884)]
    pub synth_c_arp_direction: RangedU8<3>,
    #[bits(3885..=3885)]
    pub synth_c_arp_zigzag_enabled: bool,
    #[bits(3886..=3886)]
    pub synth_c_arp_master_clock_enabled: bool,
    #[bits(3887..=3893)]
    pub synth_c_arp_rate_time: RangedU8<127>,
    #[bits(3894..=3901)]
    pub synth_c_arp_rate_time_wheel: u8,
    #[bits(3902..=3909)]
    pub synth_c_arp_rate_time_aftertouch: u8,
    #[bits(3910..=3917)]
    pub synth_c_arp_rate_time_ctrl_pedal: u8,
    #[bits(3918..=3921)]
    pub synth_c_arp_pattern_length: RangedU8<15>,
    #[bits(3922..=3953)]
    pub synth_c_arpeggiator_accent: u32,
    #[bits(3954..=3985)]
    pub synth_c_arpeggiator_gate: u32,
    #[bits(3986..=4017)]
    pub synth_c_arpeggiator_pan: u32,
    #[bits(4018..=4019)]
    pub synth_c_unison_level: RangedU8<3>,
    #[bits(4020..=4026)]
    pub synth_c_extern_cc_val1: RangedU8<127>,
    #[bits(4027..=4034)]
    pub synth_c_extern_cc_val1_wheel: u8,
    #[bits(4035..=4042)]
    pub synth_c_extern_cc_val1_aftertouch: u8,
    #[bits(4043..=4050)]
    pub synth_c_extern_cc_val1_ctrl_pedal: u8,
    #[bits(4051..=4057)]
    pub synth_c_extern_cc_val2: RangedU8<127>,
    #[bits(4058..=4065)]
    pub synth_c_extern_cc_val2_wheel: u8,
    #[bits(4066..=4073)]
    pub synth_c_extern_cc_val2_aftertouch: u8,
    #[bits(4074..=4081)]
    pub synth_c_extern_cc_val2_ctrl_pedal: u8,
    #[bits(4084..=4088)]
    pub synth_c_vibrato_delay: RangedU8<31>,
    #[bits(4130..=4131)]
    pub synth_a_analog_type_knob_1: RangedU8<3>,
    #[bits(4135..=4137)]
    pub synth_a_analog_cat_knob_2: RangedU8<7>,
    #[bits(4139..=4144)]
    pub synth_a_analog_wave_partial_knob_3: RangedU8<63>,
    #[bits(4145..=4151)]
    pub synth_a_osc_ctrl: RangedU8<127>,
    #[bits(4152..=4159)]
    pub synth_a_osc_ctrl_wheel: u8,
    #[bits(4160..=4167)]
    pub synth_a_osc_ctrl_aftertouch: u8,
    #[bits(4168..=4175)]
    pub synth_a_osc_ctrl_ctrl_pedal: u8,
    #[bits(4176..=4182)]
    pub synth_a_pitch_fine: RangedU8<127>,
    #[bits(4183..=4188)]
    pub synth_a_pitch_coarse: RangedU8<63>,
    #[bits(4191..=4197)]
    pub synth_a_osc_env_attack: RangedU8<127>,
    #[bits(4198..=4204)]
    pub synth_a_osc_env_decay: RangedU8<127>,
    #[bits(4205..=4211)]
    pub synth_a_osc_env_release: RangedU8<127>,
    #[bits(4212..=4218)]
    pub synth_a_osc_env_amount: RangedU8<127>,
    #[bits(4219..=4226)]
    pub synth_a_osc_env_amount_wheel: u8,
    #[bits(4227..=4234)]
    pub synth_a_osc_env_amount_aftertouch: u8,
    #[bits(4235..=4242)]
    pub synth_a_osc_env_amount_ctrl_pedal: u8,
    #[bits(4243..=4243)]
    pub synth_a_osc_env_to_pitch_enabled: bool,
    #[bits(4244..=4244)]
    pub synth_a_osc_env_velocity_enabled: bool,
    #[bits(4245..=4246)]
    pub synth_a_sample_options: RangedU8<3>,
    #[bits(4247..=4248)]
    pub synth_a_lfo_target: RangedU8<3>,
    #[bits(4249..=4251)]
    pub synth_a_lfo_shape: RangedU8<7>,
    #[bits(4252..=4252)]
    pub synth_a_lfo_master_clock_enabled: bool,
    #[bits(4253..=4259)]
    pub synth_a_lfo_rate_time: RangedU8<127>,
    #[bits(4260..=4267)]
    pub synth_a_lfo_rate_time_wheel: u8,
    #[bits(4268..=4275)]
    pub synth_a_lfo_rate_time_aftertouch: u8,
    #[bits(4276..=4283)]
    pub synth_a_lfo_rate_time_ctrl_pedal: u8,
    #[bits(4284..=4290)]
    pub synth_a_lfo_mod_amount: RangedU8<127>,
    #[bits(4291..=4298)]
    pub synth_a_lfo_mod_amount_wheel: u8,
    #[bits(4299..=4306)]
    pub synth_a_lfo_mod_amount_aftertouch: u8,
    #[bits(4307..=4314)]
    pub synth_a_lfo_mod_amount_ctrl_pedal: u8,
    #[bits(4315..=4321)]
    pub synth_a_amp_env_attack: RangedU8<127>,
    #[bits(4322..=4328)]
    pub synth_a_amp_env_decay: RangedU8<127>,
    #[bits(4329..=4335)]
    pub synth_a_amp_env_release: RangedU8<127>,
    #[bits(4336..=4337)]
    pub synth_a_amp_env_velocity: RangedU8<3>,
    #[bits(4338..=4340)]
    pub synth_a_filter_type: RangedU8<7>,
    #[bits(4341..=4347)]
    pub synth_a_filter_freq: RangedU8<127>,
    #[bits(4348..=4355)]
    pub synth_a_filter_freq_wheel: u8,
    #[bits(4356..=4363)]
    pub synth_a_filter_freq_aftertouch: u8,
    #[bits(4364..=4371)]
    pub synth_a_filter_freq_ctrl_pedal: u8,
    #[bits(4372..=4378)]
    pub synth_a_filter_resonance_freq_hp: RangedU8<127>,
    #[bits(4379..=4386)]
    pub synth_a_filter_resonance_wheel: u8,
    #[bits(4387..=4394)]
    pub synth_a_filter_resonance_aftertouch: u8,
    #[bits(4395..=4402)]
    pub synth_a_filter_resonance_ctrl_pedal: u8,
    #[bits(4403..=4404)]
    pub synth_a_filter_track: RangedU8<3>,
    #[bits(4405..=4406)]
    pub synth_a_filter_drive: RangedU8<3>,
    #[bits(4407..=4413)]
    pub synth_a_filter_env_amount: RangedU8<127>,
    #[bits(4414..=4421)]
    pub synth_a_filter_env_amount_wheel: u8,
    #[bits(4422..=4429)]
    pub synth_a_filter_env_amount_aftertouch: u8,
    #[bits(4430..=4437)]
    pub synth_a_filter_env_amount_ctrl_pedal: u8,
    #[bits(4438..=4444)]
    pub synth_a_filter_env_attack: RangedU8<127>,
    #[bits(4445..=4451)]
    pub synth_a_filter_env_decay: RangedU8<127>,
    #[bits(4452..=4458)]
    pub synth_a_filter_env_release: RangedU8<127>,
    #[bits(4459..=4459)]
    pub synth_a_filter_velocity_enabled: bool,
    #[bits(4460..=4460)]
    pub synth_a_filter_enabled: bool,
    #[bits(4461..=4467)]
    pub synth_a_vibrato_rate: RangedU8<127>,
    #[bits(4468..=4473)]
    pub synth_a_vibrato_amount: RangedU8<63>,
    #[bits(4474..=4474)]
    pub synth_a_sample_bright_enabled: bool,
    #[bits(4514..=4515)]
    pub synth_b_analog_type_knob_1: RangedU8<3>,
    #[bits(4519..=4521)]
    pub synth_b_analog_cat_knob_2: RangedU8<7>,
    #[bits(4523..=4528)]
    pub synth_b_analog_wave_partial_knob_3: RangedU8<63>,
    #[bits(4529..=4535)]
    pub synth_b_osc_ctrl: RangedU8<127>,
    #[bits(4536..=4543)]
    pub synth_b_osc_ctrl_wheel: u8,
    #[bits(4544..=4551)]
    pub synth_b_osc_ctrl_aftertouch: u8,
    #[bits(4552..=4559)]
    pub synth_b_osc_ctrl_ctrl_pedal: u8,
    #[bits(4560..=4566)]
    pub synth_b_pitch_fine: RangedU8<127>,
    #[bits(4567..=4572)]
    pub synth_b_pitch_coarse: RangedU8<63>,
    #[bits(4575..=4581)]
    pub synth_b_osc_env_attack: RangedU8<127>,
    #[bits(4582..=4588)]
    pub synth_b_osc_env_decay: RangedU8<127>,
    #[bits(4589..=4595)]
    pub synth_b_osc_env_release: RangedU8<127>,
    #[bits(4596..=4602)]
    pub synth_b_osc_env_amount: RangedU8<127>,
    #[bits(4603..=4610)]
    pub synth_b_osc_env_amount_wheel: u8,
    #[bits(4611..=4618)]
    pub synth_b_osc_env_amount_aftertouch: u8,
    #[bits(4619..=4626)]
    pub synth_b_osc_env_amount_ctrl_pedal: u8,
    #[bits(4627..=4627)]
    pub synth_b_osc_env_to_pitch_enabled: bool,
    #[bits(4628..=4628)]
    pub synth_b_osc_env_velocity_enabled: bool,
    #[bits(4629..=4630)]
    pub synth_b_sample_options: RangedU8<3>,
    #[bits(4631..=4632)]
    pub synth_b_lfo_target: RangedU8<3>,
    #[bits(4633..=4635)]
    pub synth_b_lfo_shape: RangedU8<7>,
    #[bits(4636..=4636)]
    pub synth_b_lfo_master_clock_enabled: bool,
    #[bits(4637..=4643)]
    pub synth_b_lfo_rate_time: RangedU8<127>,
    #[bits(4644..=4651)]
    pub synth_b_lfo_rate_time_wheel: u8,
    #[bits(4652..=4659)]
    pub synth_b_lfo_rate_time_aftertouch: u8,
    #[bits(4660..=4667)]
    pub synth_b_lfo_rate_time_ctrl_pedal: u8,
    #[bits(4668..=4674)]
    pub synth_b_lfo_mod_amount: RangedU8<127>,
    #[bits(4675..=4682)]
    pub synth_b_lfo_mod_amount_wheel: u8,
    #[bits(4683..=4690)]
    pub synth_b_lfo_mod_amount_aftertouch: u8,
    #[bits(4691..=4698)]
    pub synth_b_lfo_mod_amount_ctrl_pedal: u8,
    #[bits(4699..=4705)]
    pub synth_b_amp_env_attack: RangedU8<127>,
    #[bits(4706..=4712)]
    pub synth_b_amp_env_decay: RangedU8<127>,
    #[bits(4713..=4719)]
    pub synth_b_amp_env_release: RangedU8<127>,
    #[bits(4720..=4721)]
    pub synth_b_amp_env_velocity: RangedU8<3>,
    #[bits(4722..=4724)]
    pub synth_b_filter_type: RangedU8<7>,
    #[bits(4725..=4731)]
    pub synth_b_filter_freq: RangedU8<127>,
    #[bits(4732..=4739)]
    pub synth_b_filter_freq_wheel: u8,
    #[bits(4740..=4747)]
    pub synth_b_filter_freq_aftertouch: u8,
    #[bits(4748..=4755)]
    pub synth_b_filter_freq_ctrl_pedal: u8,
    #[bits(4756..=4762)]
    pub synth_b_filter_resonance_freq_hp: RangedU8<127>,
    #[bits(4763..=4770)]
    pub synth_b_filter_resonance_wheel: u8,
    #[bits(4771..=4778)]
    pub synth_b_filter_resonance_aftertouch: u8,
    #[bits(4779..=4786)]
    pub synth_b_filter_resonance_ctrl_pedal: u8,
    #[bits(4787..=4788)]
    pub synth_b_filter_track: RangedU8<3>,
    #[bits(4789..=4790)]
    pub synth_b_filter_drive: RangedU8<3>,
    #[bits(4791..=4797)]
    pub synth_b_filter_env_amount: RangedU8<127>,
    #[bits(4798..=4805)]
    pub synth_b_filter_env_amount_wheel: u8,
    #[bits(4806..=4813)]
    pub synth_b_filter_env_amount_aftertouch: u8,
    #[bits(4814..=4821)]
    pub synth_b_filter_env_amount_ctrl_pedal: u8,
    #[bits(4822..=4828)]
    pub synth_b_filter_env_attack: RangedU8<127>,
    #[bits(4829..=4835)]
    pub synth_b_filter_env_decay: RangedU8<127>,
    #[bits(4836..=4842)]
    pub synth_b_filter_env_release: RangedU8<127>,
    #[bits(4843..=4843)]
    pub synth_b_filter_velocity_enabled: bool,
    #[bits(4844..=4844)]
    pub synth_b_filter_enabled: bool,
    #[bits(4845..=4851)]
    pub synth_b_vibrato_rate: RangedU8<127>,
    #[bits(4852..=4857)]
    pub synth_b_vibrato_amount: RangedU8<63>,
    #[bits(4858..=4858)]
    pub synth_b_sample_bright_enabled: bool,
    #[bits(4898..=4899)]
    pub synth_c_analog_type_knob_1: RangedU8<3>,
    #[bits(4903..=4905)]
    pub synth_c_analog_cat_knob_2: RangedU8<7>,
    #[bits(4907..=4912)]
    pub synth_c_analog_wave_partial_knob_3: RangedU8<63>,
    #[bits(4913..=4919)]
    pub synth_c_osc_ctrl: RangedU8<127>,
    #[bits(4920..=4927)]
    pub synth_c_osc_ctrl_wheel: u8,
    #[bits(4928..=4935)]
    pub synth_c_osc_ctrl_aftertouch: u8,
    #[bits(4936..=4943)]
    pub synth_c_osc_ctrl_ctrl_pedal: u8,
    #[bits(4944..=4950)]
    pub synth_c_pitch_fine: RangedU8<127>,
    #[bits(4951..=4956)]
    pub synth_c_pitch_coarse: RangedU8<63>,
    #[bits(4959..=4965)]
    pub synth_c_osc_env_attack: RangedU8<127>,
    #[bits(4966..=4972)]
    pub synth_c_osc_env_decay: RangedU8<127>,
    #[bits(4973..=4979)]
    pub synth_c_osc_env_release: RangedU8<127>,
    #[bits(4980..=4986)]
    pub synth_c_osc_env_amount: RangedU8<127>,
    #[bits(4987..=4994)]
    pub synth_c_osc_env_amount_wheel: u8,
    #[bits(4995..=5002)]
    pub synth_c_osc_env_amount_aftertouch: u8,
    #[bits(5003..=5010)]
    pub synth_c_osc_env_amount_ctrl_pedal: u8,
    #[bits(5011..=5011)]
    pub synth_c_osc_env_to_pitch_enabled: bool,
    #[bits(5012..=5012)]
    pub synth_c_osc_env_velocity_enabled: bool,
    #[bits(5013..=5014)]
    pub synth_c_sample_options: RangedU8<3>,
    #[bits(5015..=5016)]
    pub synth_c_lfo_target: RangedU8<3>,
    #[bits(5017..=5019)]
    pub synth_c_lfo_shape: RangedU8<7>,
    #[bits(5020..=5020)]
    pub synth_c_lfo_master_clock_enabled: bool,
    #[bits(5021..=5027)]
    pub synth_c_lfo_rate_time: RangedU8<127>,
    #[bits(5028..=5035)]
    pub synth_c_lfo_rate_time_wheel: u8,
    #[bits(5036..=5043)]
    pub synth_c_lfo_rate_time_aftertouch: u8,
    #[bits(5044..=5051)]
    pub synth_c_lfo_rate_time_ctrl_pedal: u8,
    #[bits(5052..=5058)]
    pub synth_c_lfo_mod_amount: RangedU8<127>,
    #[bits(5059..=5066)]
    pub synth_c_lfo_mod_amount_wheel: u8,
    #[bits(5067..=5074)]
    pub synth_c_lfo_mod_amount_aftertouch: u8,
    #[bits(5075..=5082)]
    pub synth_c_lfo_mod_amount_ctrl_pedal: u8,
    #[bits(5083..=5089)]
    pub synth_c_amp_env_attack: RangedU8<127>,
    #[bits(5090..=5096)]
    pub synth_c_amp_env_decay: RangedU8<127>,
    #[bits(5097..=5103)]
    pub synth_c_amp_env_release: RangedU8<127>,
    #[bits(5104..=5105)]
    pub synth_c_amp_env_velocity: RangedU8<3>,
    #[bits(5106..=5108)]
    pub synth_c_filter_type: RangedU8<7>,
    #[bits(5109..=5115)]
    pub synth_c_filter_freq: RangedU8<127>,
    #[bits(5116..=5123)]
    pub synth_c_filter_freq_wheel: u8,
    #[bits(5124..=5131)]
    pub synth_c_filter_freq_aftertouch: u8,
    #[bits(5132..=5139)]
    pub synth_c_filter_freq_ctrl_pedal: u8,
    #[bits(5140..=5146)]
    pub synth_c_filter_resonance_freq_hp: RangedU8<127>,
    #[bits(5147..=5154)]
    pub synth_c_filter_resonance_wheel: u8,
    #[bits(5155..=5162)]
    pub synth_c_filter_resonance_aftertouch: u8,
    #[bits(5163..=5170)]
    pub synth_c_filter_resonance_ctrl_pedal: u8,
    #[bits(5171..=5172)]
    pub synth_c_filter_track: RangedU8<3>,
    #[bits(5173..=5174)]
    pub synth_c_filter_drive: RangedU8<3>,
    #[bits(5175..=5181)]
    pub synth_c_filter_env_amount: RangedU8<127>,
    #[bits(5182..=5189)]
    pub synth_c_filter_env_amount_wheel: u8,
    #[bits(5190..=5197)]
    pub synth_c_filter_env_amount_aftertouch: u8,
    #[bits(5198..=5205)]
    pub synth_c_filter_env_amount_ctrl_pedal: u8,
    #[bits(5206..=5212)]
    pub synth_c_filter_env_attack: RangedU8<127>,
    #[bits(5213..=5219)]
    pub synth_c_filter_env_decay: RangedU8<127>,
    #[bits(5220..=5226)]
    pub synth_c_filter_env_release: RangedU8<127>,
    #[bits(5227..=5227)]
    pub synth_c_filter_velocity_enabled: bool,
    #[bits(5228..=5228)]
    pub synth_c_filter_enabled: bool,
    #[bits(5229..=5235)]
    pub synth_c_vibrato_rate: RangedU8<127>,
    #[bits(5236..=5241)]
    pub synth_c_vibrato_amount: RangedU8<63>,
    #[bits(5242..=5242)]
    pub synth_c_sample_bright_enabled: bool,
    #[bits(5280..=5280)]
    pub synth_a_fx_mod_1_enabled: bool,
    #[bits(5281..=5281)]
    pub synth_a_fx_mod_1_master_clock_enabled: bool,
    #[bits(5282..=5288)]
    pub synth_a_fx_mod_1_rate: RangedU8<127>,
    #[bits(5289..=5296)]
    pub synth_a_fx_mod_1_rate_wheel: u8,
    #[bits(5297..=5304)]
    pub synth_a_fx_mod_1_rate_aftertouch: u8,
    #[bits(5305..=5312)]
    pub synth_a_fx_mod_1_rate_ctrl_pedal: u8,
    #[bits(5313..=5319)]
    pub synth_a_fx_mod_1_amount: RangedU8<127>,
    #[bits(5320..=5327)]
    pub synth_a_fx_mod_1_amount_wheel: u8,
    #[bits(5328..=5335)]
    pub synth_a_fx_mod_1_amount_aftertouch: u8,
    #[bits(5336..=5343)]
    pub synth_a_fx_mod_1_amount_ctrl_pedal: u8,
    #[bits(5344..=5347)]
    pub synth_a_fx_mod_1_mode: RangedU8<15>,
    #[bits(5348..=5348)]
    pub synth_a_fx_mod_2_enabled: bool,
    #[bits(5349..=5355)]
    pub synth_a_fx_mod_2_rate: RangedU8<127>,
    #[bits(5356..=5363)]
    pub synth_a_fx_mod_2_rate_wheel: u8,
    #[bits(5364..=5371)]
    pub synth_a_fx_mod_2_rate_aftertouch: u8,
    #[bits(5372..=5379)]
    pub synth_a_fx_mod_2_rate_ctrl_pedal: u8,
    #[bits(5380..=5386)]
    pub synth_a_fx_mod_2_amount: RangedU8<127>,
    #[bits(5387..=5394)]
    pub synth_a_fx_mod_2_amount_wheel: u8,
    #[bits(5395..=5402)]
    pub synth_a_fx_mod_2_amount_aftertouch: u8,
    #[bits(5403..=5410)]
    pub synth_a_fx_mod_2_amount_ctrl_pedal: u8,
    #[bits(5411..=5414)]
    pub synth_a_fx_mod_2_mode: RangedU8<15>,
    #[bits(5415..=5415)]
    pub synth_a_fx_amp_sim_eq_enabled: bool,
    #[bits(5416..=5422)]
    pub synth_a_fx_amp_sim_eq_treb: RangedU8<127>,
    #[bits(5423..=5429)]
    pub synth_a_fx_amp_sim_eq_mid: RangedU8<127>,
    #[bits(5430..=5436)]
    pub synth_a_fx_amp_sim_eq_bass: RangedU8<127>,
    #[bits(5437..=5443)]
    pub synth_a_fx_amp_sim_eq_freq: RangedU8<127>,
    #[bits(5444..=5451)]
    pub synth_a_fx_amp_sim_eq_freq_wheel: u8,
    #[bits(5452..=5459)]
    pub synth_a_fx_amp_sim_eq_freq_aftertouch: u8,
    #[bits(5460..=5467)]
    pub synth_a_fx_amp_sim_eq_freq_ctrl_pedal: u8,
    #[bits(5468..=5474)]
    pub synth_a_fx_amp_sim_eq_drive: RangedU8<127>,
    #[bits(5475..=5482)]
    pub synth_a_fx_amp_sim_eq_drive_wheel: u8,
    #[bits(5483..=5490)]
    pub synth_a_fx_amp_sim_eq_drive_aftertouch: u8,
    #[bits(5491..=5498)]
    pub synth_a_fx_amp_sim_eq_drive_ctrl_pedal: u8,
    #[bits(5503..=5503)]
    pub synth_a_fx_comp_enabled: bool,
    #[bits(5504..=5510)]
    pub synth_a_fx_comp_amount: RangedU8<127>,
    #[bits(5511..=5511)]
    pub synth_a_fx_comp_response: bool,
    #[bits(5512..=5512)]
    pub synth_a_fx_delay_enabled: bool,
    #[bits(5513..=5513)]
    pub synth_a_fx_delay_tempo_master_clock_enabled: bool,
    #[bits(5514..=5520)]
    pub synth_a_fx_delay_tempo: RangedU8<127>,
    #[bits(5528..=5535)]
    pub synth_a_fx_delay_tempo_wheel: u8,
    #[bits(5536..=5543)]
    pub synth_a_fx_delay_tempo_aftertouch: u8,
    #[bits(5544..=5551)]
    pub synth_a_fx_delay_tempo_ctrl_pedal: u8,
    #[bits(5573..=5579)]
    pub synth_a_fx_delay_mix: RangedU8<127>,
    #[bits(5580..=5587)]
    pub synth_a_fx_delay_mix_wheel: u8,
    #[bits(5588..=5595)]
    pub synth_a_fx_delay_mix_aftertouch: u8,
    #[bits(5596..=5603)]
    pub synth_a_fx_delay_mix_ctrl_pedal: u8,
    #[bits(5604..=5604)]
    pub synth_a_fx_delay_normal_analog: bool,
    #[bits(5605..=5605)]
    pub synth_a_fx_delay_ping_pong_enabled: bool,
    #[bits(5606..=5607)]
    pub synth_a_fx_delay_filter_type: RangedU8<3>,
    #[bits(5608..=5614)]
    pub synth_a_fx_delay_feedback: RangedU8<127>,
    #[bits(5615..=5622)]
    pub synth_a_fx_delay_feedback_wheel: u8,
    #[bits(5623..=5630)]
    pub synth_a_fx_delay_feedback_aftertouch: u8,
    #[bits(5631..=5638)]
    pub synth_a_fx_delay_feedback_ctrl_pedal: u8,
    #[bits(5639..=5642)]
    pub synth_a_fx_delay_effects: RangedU8<15>,
    #[bits(5643..=5643)]
    pub synth_a_fx_reverb_enabled: bool,
    #[bits(5644..=5650)]
    pub synth_a_fx_reverb_amount: RangedU8<127>,
    #[bits(5651..=5658)]
    pub synth_a_fx_reverb_amount_wheel: u8,
    #[bits(5659..=5666)]
    pub synth_a_fx_reverb_amount_aftertouch: u8,
    #[bits(5667..=5674)]
    pub synth_a_fx_reverb_amount_ctrl_pedal: u8,
    #[bits(5675..=5676)]
    pub synth_a_fx_reverb_dark_bright: RangedU8<3>,
    #[bits(5677..=5680)]
    pub synth_a_fx_reverb_type: RangedU8<15>,
    #[bits(5685..=5688)]
    pub synth_a_fx_amp_sim_eq_mode: RangedU8<15>,
    #[bits(5720..=5720)]
    pub synth_b_fx_mod_1_enabled: bool,
    #[bits(5721..=5721)]
    pub synth_b_fx_mod_1_master_clock_enabled: bool,
    #[bits(5722..=5728)]
    pub synth_b_fx_mod_1_rate: RangedU8<127>,
    #[bits(5729..=5736)]
    pub synth_b_fx_mod_1_rate_wheel: u8,
    #[bits(5737..=5744)]
    pub synth_b_fx_mod_1_rate_aftertouch: u8,
    #[bits(5745..=5752)]
    pub synth_b_fx_mod_1_rate_ctrl_pedal: u8,
    #[bits(5753..=5759)]
    pub synth_b_fx_mod_1_amount: RangedU8<127>,
    #[bits(5760..=5767)]
    pub synth_b_fx_mod_1_amount_wheel: u8,
    #[bits(5768..=5775)]
    pub synth_b_fx_mod_1_amount_aftertouch: u8,
    #[bits(5776..=5783)]
    pub synth_b_fx_mod_1_amount_ctrl_pedal: u8,
    #[bits(5784..=5787)]
    pub synth_b_fx_mod_1_mode: RangedU8<15>,
    #[bits(5788..=5788)]
    pub synth_b_fx_mod_2_enabled: bool,
    #[bits(5789..=5795)]
    pub synth_b_fx_mod_2_rate: RangedU8<127>,
    #[bits(5796..=5803)]
    pub synth_b_fx_mod_2_rate_wheel: u8,
    #[bits(5804..=5811)]
    pub synth_b_fx_mod_2_rate_aftertouch: u8,
    #[bits(5812..=5819)]
    pub synth_b_fx_mod_2_rate_ctrl_pedal: u8,
    #[bits(5820..=5826)]
    pub synth_b_fx_mod_2_amount: RangedU8<127>,
    #[bits(5827..=5834)]
    pub synth_b_fx_mod_2_amount_wheel: u8,
    #[bits(5835..=5842)]
    pub synth_b_fx_mod_2_amount_aftertouch: u8,
    #[bits(5843..=5850)]
    pub synth_b_fx_mod_2_amount_ctrl_pedal: u8,
    #[bits(5851..=5854)]
    pub synth_b_fx_mod_2_mode: RangedU8<15>,
    #[bits(5855..=5855)]
    pub synth_b_fx_amp_sim_eq_enabled: bool,
    #[bits(5856..=5862)]
    pub synth_b_fx_amp_sim_eq_treb: RangedU8<127>,
    #[bits(5863..=5869)]
    pub synth_b_fx_amp_sim_eq_mid: RangedU8<127>,
    #[bits(5870..=5876)]
    pub synth_b_fx_amp_sim_eq_bass: RangedU8<127>,
    #[bits(5877..=5883)]
    pub synth_b_fx_amp_sim_eq_freq: RangedU8<127>,
    #[bits(5884..=5891)]
    pub synth_b_fx_amp_sim_eq_freq_wheel: u8,
    #[bits(5892..=5899)]
    pub synth_b_fx_amp_sim_eq_freq_aftertouch: u8,
    #[bits(5900..=5907)]
    pub synth_b_fx_amp_sim_eq_freq_ctrl_pedal: u8,
    #[bits(5908..=5914)]
    pub synth_b_fx_amp_sim_eq_drive: RangedU8<127>,
    #[bits(5915..=5922)]
    pub synth_b_fx_amp_sim_eq_drive_wheel: u8,
    #[bits(5923..=5930)]
    pub synth_b_fx_amp_sim_eq_drive_aftertouch: u8,
    #[bits(5931..=5938)]
    pub synth_b_fx_amp_sim_eq_drive_ctrl_pedal: u8,
    #[bits(5943..=5943)]
    pub synth_b_fx_comp_enabled: bool,
    #[bits(5944..=5950)]
    pub synth_b_fx_comp_amount: RangedU8<127>,
    #[bits(5951..=5951)]
    pub synth_b_fx_comp_response: bool,
    #[bits(5952..=5952)]
    pub synth_b_fx_delay_enabled: bool,
    #[bits(5953..=5953)]
    pub synth_b_fx_delay_tempo_master_clock_enabled: bool,
    #[bits(5954..=5960)]
    pub synth_b_fx_delay_tempo: RangedU8<127>,
    #[bits(5968..=5975)]
    pub synth_b_fx_delay_tempo_wheel: u8,
    #[bits(5976..=5983)]
    pub synth_b_fx_delay_tempo_aftertouch: u8,
    #[bits(5984..=5991)]
    pub synth_b_fx_delay_tempo_ctrl_pedal: u8,
    #[bits(6013..=6019)]
    pub synth_b_fx_delay_mix: RangedU8<127>,
    #[bits(6020..=6027)]
    pub synth_b_fx_delay_mix_wheel: u8,
    #[bits(6028..=6035)]
    pub synth_b_fx_delay_mix_aftertouch: u8,
    #[bits(6036..=6043)]
    pub synth_b_fx_delay_mix_ctrl_pedal: u8,
    #[bits(6044..=6044)]
    pub synth_b_fx_delay_normal_analog: bool,
    #[bits(6045..=6045)]
    pub synth_b_fx_delay_ping_pong_enabled: bool,
    #[bits(6046..=6047)]
    pub synth_b_fx_delay_filter_type: RangedU8<3>,
    #[bits(6048..=6054)]
    pub synth_b_fx_delay_feedback: RangedU8<127>,
    #[bits(6055..=6062)]
    pub synth_b_fx_delay_feedback_wheel: u8,
    #[bits(6063..=6070)]
    pub synth_b_fx_delay_feedback_aftertouch: u8,
    #[bits(6071..=6078)]
    pub synth_b_fx_delay_feedback_ctrl_pedal: u8,
    #[bits(6079..=6082)]
    pub synth_b_fx_delay_effects: RangedU8<15>,
    #[bits(6083..=6083)]
    pub synth_b_fx_reverb_enabled: bool,
    #[bits(6084..=6090)]
    pub synth_b_fx_reverb_amount: RangedU8<127>,
    #[bits(6091..=6098)]
    pub synth_b_fx_reverb_amount_wheel: u8,
    #[bits(6099..=6106)]
    pub synth_b_fx_reverb_amount_aftertouch: u8,
    #[bits(6107..=6114)]
    pub synth_b_fx_reverb_amount_ctrl_pedal: u8,
    #[bits(6115..=6116)]
    pub synth_b_fx_reverb_dark_bright: RangedU8<3>,
    #[bits(6117..=6120)]
    pub synth_b_fx_reverb_type: RangedU8<15>,
    #[bits(6125..=6128)]
    pub synth_b_fx_amp_sim_eq_mode: RangedU8<15>,
    #[bits(6160..=6160)]
    pub synth_c_fx_mod_1_enabled: bool,
    #[bits(6161..=6161)]
    pub synth_c_fx_mod_1_master_clock_enabled: bool,
    #[bits(6162..=6168)]
    pub synth_c_fx_mod_1_rate: RangedU8<127>,
    #[bits(6169..=6176)]
    pub synth_c_fx_mod_1_rate_wheel: u8,
    #[bits(6177..=6184)]
    pub synth_c_fx_mod_1_rate_aftertouch: u8,
    #[bits(6185..=6192)]
    pub synth_c_fx_mod_1_rate_ctrl_pedal: u8,
    #[bits(6193..=6199)]
    pub synth_c_fx_mod_1_amount: RangedU8<127>,
    #[bits(6200..=6207)]
    pub synth_c_fx_mod_1_amount_wheel: u8,
    #[bits(6208..=6215)]
    pub synth_c_fx_mod_1_amount_aftertouch: u8,
    #[bits(6216..=6223)]
    pub synth_c_fx_mod_1_amount_ctrl_pedal: u8,
    #[bits(6224..=6227)]
    pub synth_c_fx_mod_1_mode: RangedU8<15>,
    #[bits(6228..=6228)]
    pub synth_c_fx_mod_2_enabled: bool,
    #[bits(6229..=6235)]
    pub synth_c_fx_mod_2_rate: RangedU8<127>,
    #[bits(6236..=6243)]
    pub synth_c_fx_mod_2_rate_wheel: u8,
    #[bits(6244..=6251)]
    pub synth_c_fx_mod_2_rate_aftertouch: u8,
    #[bits(6252..=6259)]
    pub synth_c_fx_mod_2_rate_ctrl_pedal: u8,
    #[bits(6260..=6266)]
    pub synth_c_fx_mod_2_amount: RangedU8<127>,
    #[bits(6267..=6274)]
    pub synth_c_fx_mod_2_amount_wheel: u8,
    #[bits(6275..=6282)]
    pub synth_c_fx_mod_2_amount_aftertouch: u8,
    #[bits(6283..=6290)]
    pub synth_c_fx_mod_2_amount_ctrl_pedal: u8,
    #[bits(6291..=6294)]
    pub synth_c_fx_mod_2_mode: RangedU8<15>,
    #[bits(6295..=6295)]
    pub synth_c_fx_amp_sim_eq_enabled: bool,
    #[bits(6296..=6302)]
    pub synth_c_fx_amp_sim_eq_treb: RangedU8<127>,
    #[bits(6303..=6309)]
    pub synth_c_fx_amp_sim_eq_mid: RangedU8<127>,
    #[bits(6310..=6316)]
    pub synth_c_fx_amp_sim_eq_bass: RangedU8<127>,
    #[bits(6317..=6323)]
    pub synth_c_fx_amp_sim_eq_freq: RangedU8<127>,
    #[bits(6324..=6331)]
    pub synth_c_fx_amp_sim_eq_freq_wheel: u8,
    #[bits(6332..=6339)]
    pub synth_c_fx_amp_sim_eq_freq_aftertouch: u8,
    #[bits(6340..=6347)]
    pub synth_c_fx_amp_sim_eq_freq_ctrl_pedal: u8,
    #[bits(6348..=6354)]
    pub synth_c_fx_amp_sim_eq_drive: RangedU8<127>,
    #[bits(6355..=6362)]
    pub synth_c_fx_amp_sim_eq_drive_wheel: u8,
    #[bits(6363..=6370)]
    pub synth_c_fx_amp_sim_eq_drive_aftertouch: u8,
    #[bits(6371..=6378)]
    pub synth_c_fx_amp_sim_eq_drive_ctrl_pedal: u8,
    #[bits(6383..=6383)]
    pub synth_c_fx_comp_enabled: bool,
    #[bits(6384..=6390)]
    pub synth_c_fx_comp_amount: RangedU8<127>,
    #[bits(6391..=6391)]
    pub synth_c_fx_comp_response: bool,
    #[bits(6392..=6392)]
    pub synth_c_fx_delay_enabled: bool,
    #[bits(6393..=6393)]
    pub synth_c_fx_delay_tempo_master_clock_enabled: bool,
    #[bits(6394..=6400)]
    pub synth_c_fx_delay_tempo: RangedU8<127>,
    #[bits(6408..=6415)]
    pub synth_c_fx_delay_tempo_wheel: u8,
    #[bits(6416..=6423)]
    pub synth_c_fx_delay_tempo_aftertouch: u8,
    #[bits(6424..=6431)]
    pub synth_c_fx_delay_tempo_ctrl_pedal: u8,
    #[bits(6453..=6459)]
    pub synth_c_fx_delay_mix: RangedU8<127>,
    #[bits(6460..=6467)]
    pub synth_c_fx_delay_mix_wheel: u8,
    #[bits(6468..=6475)]
    pub synth_c_fx_delay_mix_aftertouch: u8,
    #[bits(6476..=6483)]
    pub synth_c_fx_delay_mix_ctrl_pedal: u8,
    #[bits(6484..=6484)]
    pub synth_c_fx_delay_normal_analog: bool,
    #[bits(6485..=6485)]
    pub synth_c_fx_delay_ping_pong_enabled: bool,
    #[bits(6486..=6487)]
    pub synth_c_fx_delay_filter_type: RangedU8<3>,
    #[bits(6488..=6494)]
    pub synth_c_fx_delay_feedback: RangedU8<127>,
    #[bits(6495..=6502)]
    pub synth_c_fx_delay_feedback_wheel: u8,
    #[bits(6503..=6510)]
    pub synth_c_fx_delay_feedback_aftertouch: u8,
    #[bits(6511..=6518)]
    pub synth_c_fx_delay_feedback_ctrl_pedal: u8,
    #[bits(6519..=6522)]
    pub synth_c_fx_delay_effects: RangedU8<15>,
    #[bits(6523..=6523)]
    pub synth_c_fx_reverb_enabled: bool,
    #[bits(6524..=6530)]
    pub synth_c_fx_reverb_amount: RangedU8<127>,
    #[bits(6531..=6538)]
    pub synth_c_fx_reverb_amount_wheel: u8,
    #[bits(6539..=6546)]
    pub synth_c_fx_reverb_amount_aftertouch: u8,
    #[bits(6547..=6554)]
    pub synth_c_fx_reverb_amount_ctrl_pedal: u8,
    #[bits(6555..=6556)]
    pub synth_c_fx_reverb_dark_bright: RangedU8<3>,
    #[bits(6557..=6560)]
    pub synth_c_fx_reverb_type: RangedU8<15>,
    #[bits(6565..=6568)]
    pub synth_c_fx_amp_sim_eq_mode: RangedU8<15>,
}

/// The `(bank, location)` pair from the header, uninterpreted: bank 0..=5 for
/// the six program banks and location 0..=63 on current exports. Not validated
/// — see the Stage 3's note on out-of-range locations in old files.
pub fn location(file: &Cbin<Program>) -> (u16, u16) {
    file.header.slot()
}

pub fn read_from(reader: &mut (impl Read + Seek)) -> Result<Cbin<Program>, Error> {
    let file: Cbin<Program> = cbin::read(reader, FORMAT)?;
    crate::formats::known_version(FORMAT, file.header.version, KNOWN_VERSIONS)?;
    Ok(file)
}
