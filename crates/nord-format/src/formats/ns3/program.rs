//! The Stage 3 program body (`.ns3f`, `.ns3l`): 548 bytes, every documented
//! parameter placed.
//!
//! The program-wide globals were decoded first and by hand; everything else — the
//! organ's two presets and their drawbars, the piano, synth, extern and the whole
//! effects chain — comes from the byte maps. Where the two disagree the hand
//! decode wins: the map has one 22-bit `split` run where the companion doc, and
//! this module, break it into eight fields.
//!
//! Values are raw except where the documentation enumerates them; see [the module
//! docs](super) for what that ceiling is and why.

use crate::cbin::{self, Cbin, Header};
use crate::components::{
    sparse_enum, Effect1Type, Effect2Type, MasterTempo, ProgramCategory, ReverbType, SplitNote,
    SplitWidth, StageTranspose,
};
use crate::error::Error;
use crate::types::{RangedU16, RangedU8};
use std::io::{Read, Seek};

pub const FORMAT: &str = "ns3f";
/// Schema versions this build's field offsets have been validated against:
/// program v3.00 (OS v0.92) through v3.04 (OS v2.10 and later), stored ×100.
pub const KNOWN_VERSIONS: &[u32] = &[300, 301, 302, 303, 304];
pub const BODY_LEN: usize = 548;

sparse_enum!(
    /// Which panels the program enables.
    PanelEnable, 2, {
        0 => AOnly, "A only";
        1 => BOnly, "B only";
        2 => Both, "A & B";
    }
);

sparse_enum!(
    /// What the second keyboard plays when Dual Keyboard is on.
    DualKeyboardStyle, 2, {
        0 => Panel, "Panel";
        1 => Organ, "Organ";
        2 => Piano, "Piano";
        3 => Synth, "Synth";
    }
);

/// The program-wide globals at the head of the body. Bits are MSB-first from body
/// byte 0 (`0x2c` in a type-1 file), so byte 0x05 bit 7 is bit 40.
///
/// ⚠️ The three split notes can be stored out of order — the panel reorders them on
/// display (documented with specimens in the ns3-program-viewer sources). The
/// decode reports what is stored.
#[nord_bits_derive::bitbody(548)]
pub struct Program {
    /// Body byte 0x05 bit 7: which panel was selected when the program was stored.
    #[bits(40..=40)]
    pub panel_b_selected: bool,
    #[bits(41..=42)]
    pub panel_enable: PanelEnable,
    #[bits(43..=43)]
    pub split_enabled: bool,
    #[bits(44..=44)]
    pub split_low_enabled: bool,
    #[bits(45..=45)]
    pub split_mid_enabled: bool,
    #[bits(46..=46)]
    pub split_high_enabled: bool,
    #[bits(47..=50)]
    pub split_low_note: SplitNote,
    #[bits(51..=54)]
    pub split_mid_note: SplitNote,
    #[bits(55..=58)]
    pub split_high_note: SplitNote,
    #[bits(59..=60)]
    pub split_low_width: SplitWidth,
    #[bits(61..=62)]
    pub split_mid_width: SplitWidth,
    #[bits(63..=64)]
    pub split_high_width: SplitWidth,
    /// Touched at least once, not active: the untouched default stores 6 (= 0).
    #[bits(96..=96)]
    pub transpose_enabled: bool,
    #[bits(97..=100)]
    pub transpose: StageTranspose,
    #[bits(101..=108)]
    pub master_clock: MasterTempo,
    #[bits(116..=116)]
    pub dual_keyboard: bool,
    #[bits(118..=119)]
    pub dual_keyboard_style: DualKeyboardStyle,
    // ── Generated from the Stage byte maps. Everything below is one
    //    field per documented run, in offset order.
    #[bits(65..=66)]
    pub piano_layer_detune: PianoLayerDetune,
    #[bits(67..=67)]
    pub organ_pitch_stick: bool,
    #[bits(68..=70)]
    pub organ_vibrato_mode: OrganVibratoMode,
    #[bits(71..=71)]
    pub rotary_speaker_speed: bool,
    #[bits(72..=72)]
    pub rotary_speaker_stop_mode: bool,
    #[bits(73..=75)]
    pub rotary_speaker_speed_wheel: RangedU8<7>,
    #[bits(76..=78)]
    pub rotary_speaker_speed_aftertouch: RangedU8<7>,
    #[bits(79..=81)]
    pub rotary_speaker_speed_ctrl_pedal: RangedU8<7>,
    #[bits(109..=115)]
    pub rotary_speaker_drive: RangedU8<127>,
    #[bits(120..=123)]
    pub synth_pitch_stick_range: RangedU8<15>,
    #[bits(184..=184)]
    pub piano_on: bool,
    #[bits(185..=188)]
    pub piano_kb_zone: RangedU8<15>,
    #[bits(189..=195)]
    pub piano_volume: RangedU8<127>,
    #[bits(196..=203)]
    pub piano_volume_wheel: u8,
    #[bits(204..=211)]
    pub piano_volume_aftertouch: u8,
    #[bits(212..=219)]
    pub piano_volume_ctrl_pedal: u8,
    #[bits(220..=223)]
    pub piano_octave_shift: RangedU8<15>,
    #[bits(224..=224)]
    pub piano_pitch_stick: bool,
    #[bits(225..=225)]
    pub piano_sustain_pedal: bool,
    #[bits(226..=228)]
    pub piano_type: PianoType,
    #[bits(229..=233)]
    pub piano_model: RangedU8<31>,
    #[bits(234..=235)]
    pub clavinet_model: ClavinetModel,
    #[bits(236..=267)]
    pub piano_sample_name: u32,
    #[bits(268..=268)]
    pub piano_soft_release: bool,
    #[bits(269..=269)]
    pub piano_string_resonance: bool,
    #[bits(270..=270)]
    pub piano_pedal_noise: bool,
    #[bits(271..=272)]
    pub piano_kb_touch: PianoKbTouch,
    #[bits(274..=276)]
    pub piano_timbre: PianoTimbre,
    #[bits(304..=304)]
    pub synth_on: bool,
    #[bits(305..=308)]
    pub synth_kb_zone: RangedU8<15>,
    #[bits(309..=315)]
    pub synth_volume: RangedU8<127>,
    #[bits(316..=323)]
    pub synth_volume_wheel: u8,
    #[bits(324..=331)]
    pub synth_volume_aftertouch: u8,
    #[bits(332..=339)]
    pub synth_volume_ctrl_pedal: u8,
    #[bits(340..=343)]
    pub synth_octave_shift: RangedU8<15>,
    #[bits(344..=344)]
    pub synth_pitch_stick: bool,
    #[bits(345..=345)]
    pub synth_sustain_pedal: bool,
    #[bits(346..=355)]
    pub synth_preset_location: RangedU16<1023>,
    #[bits(672..=672)]
    pub synth_kh_hold: bool,
    #[bits(673..=673)]
    pub synth_arp_on: bool,
    #[bits(674..=674)]
    pub synth_arp_kb_sync: bool,
    #[bits(675..=676)]
    pub synth_arp_range: SynthArpRange,
    #[bits(677..=678)]
    pub synth_arp_pattern: SynthArpPattern,
    #[bits(679..=679)]
    pub synth_arp_master_clock: bool,
    #[bits(680..=686)]
    pub synth_arp_rate: RangedU8<127>,
    #[bits(687..=694)]
    pub synth_arp_rate_wheel: u8,
    #[bits(695..=702)]
    pub synth_arp_rate_aftertouch: u8,
    #[bits(703..=710)]
    pub synth_arp_rate_ctrl_pedal: u8,
    #[bits(711..=712)]
    pub synth_voice: SynthVoice,
    #[bits(713..=719)]
    pub synth_glide: RangedU8<127>,
    #[bits(720..=721)]
    pub synth_unison: SynthUnison,
    #[bits(722..=724)]
    pub synth_vibrato: SynthVibrato,
    #[bits(725..=727)]
    pub synth_lfo_wave: SynthLfoWave,
    #[bits(728..=728)]
    pub synth_lfo_master_clock: bool,
    #[bits(729..=735)]
    pub synth_lfo_rate: RangedU8<127>,
    #[bits(736..=743)]
    pub synth_lfo_rate_wheel: u8,
    #[bits(744..=751)]
    pub synth_lfo_rate_aftertouch: u8,
    #[bits(752..=759)]
    pub synth_lfo_rate_ctrl_pedal: u8,
    #[bits(760..=766)]
    pub synth_mod_env_attack: RangedU8<127>,
    #[bits(767..=773)]
    pub synth_mod_env_decay: RangedU8<127>,
    #[bits(774..=780)]
    pub synth_mod_env_release: RangedU8<127>,
    #[bits(781..=781)]
    pub synth_mod_env_velocity: bool,
    #[bits(782..=784)]
    pub synth_oscillator_type: SynthOscillatorType,
    #[bits(785..=793)]
    pub synth_oscillator_1_wave_form: RangedU16<511>,
    #[bits(795..=798)]
    pub synth_oscillator_config: SynthOscillatorConfig,
    #[bits(799..=804)]
    pub synth_pitch: RangedU8<63>,
    #[bits(805..=811)]
    pub synth_oscillator_control: RangedU8<127>,
    #[bits(812..=819)]
    pub synth_oscillator_control_wheel: u8,
    #[bits(820..=827)]
    pub synth_oscillator_control_aftertouch: u8,
    #[bits(828..=835)]
    pub synth_oscillator_control_ctrl_pedal: u8,
    #[bits(836..=842)]
    pub synth_oscillator_mod: RangedU8<127>,
    #[bits(843..=850)]
    pub synth_oscillator_mod_wheel: u8,
    #[bits(851..=858)]
    pub synth_oscillator_mod_aftertouch: u8,
    #[bits(859..=866)]
    pub synth_oscillator_mod_ctrl_pedal: u8,
    #[bits(867..=869)]
    pub synth_filter_type: SynthFilterType,
    #[bits(870..=876)]
    pub synth_filter_freq: RangedU8<127>,
    #[bits(877..=884)]
    pub synth_filter_freq_wheel: u8,
    #[bits(885..=892)]
    pub synth_filter_freq_aftertouch: u8,
    #[bits(893..=900)]
    pub synth_filter_freq_ctrl_pedal: u8,
    #[bits(901..=907)]
    pub synth_filter_hp_freq_res: RangedU8<127>,
    #[bits(908..=915)]
    pub synth_filter_hp_freq_res_wheel: u8,
    #[bits(916..=923)]
    pub synth_filter_hp_freq_res_aftertouch: u8,
    #[bits(924..=931)]
    pub synth_filter_hp_freq_res_ctrl_pedal: u8,
    #[bits(932..=938)]
    pub synth_filter_lfo_amount: RangedU8<127>,
    #[bits(939..=946)]
    pub synth_filter_lfo_amount_wheel: u8,
    #[bits(947..=954)]
    pub synth_filter_lfo_amount_aftertouch: u8,
    #[bits(955..=962)]
    pub synth_filter_lfo_amount_ctrl_pedal: u8,
    #[bits(963..=969)]
    pub synth_filter_vel_mod_env_amount: RangedU8<127>,
    #[bits(970..=971)]
    pub synth_filter_kb_track: SynthFilterKbTrack,
    #[bits(972..=973)]
    pub synth_filter_drive: SynthFilterDrive,
    #[bits(974..=980)]
    pub synth_amp_env_attack: RangedU8<127>,
    #[bits(981..=987)]
    pub synth_amp_env_decay: RangedU8<127>,
    #[bits(988..=994)]
    pub synth_amp_env_release: RangedU8<127>,
    #[bits(995..=996)]
    pub synth_amp_env_velocity: SynthAmpEnvVelocity,
    #[bits(997..=1028)]
    pub synth_sample_id: u32,
    #[bits(1029..=1029)]
    pub synth_fast_attack: bool,
    #[bits(1104..=1104)]
    pub organ_on: bool,
    #[bits(1105..=1108)]
    pub organ_kb_zone: OrganKbZone,
    #[bits(1109..=1115)]
    pub organ_volume: RangedU8<127>,
    #[bits(1116..=1123)]
    pub organ_volume_wheel: u8,
    #[bits(1124..=1131)]
    pub organ_volume_aftertouch: u8,
    #[bits(1132..=1139)]
    pub organ_volume_ctrl_pedal: u8,
    #[bits(1140..=1143)]
    pub organ_octave_shift: RangedU8<15>,
    #[bits(1144..=1144)]
    pub organ_sustain_pedal: bool,
    #[bits(1145..=1147)]
    pub organ_type: OrganType,
    #[bits(1148..=1148)]
    pub organ_live_mode: bool,
    #[bits(1149..=1149)]
    pub organ_preset_2_on: bool,
    #[bits(1168..=1171)]
    pub organ_preset_1_drawbar_1: RangedU8<15>,
    #[bits(1172..=1176)]
    pub organ_preset_1_drawbar_1_wheel: RangedU8<31>,
    #[bits(1177..=1181)]
    pub organ_preset_1_drawbar_1_aftertouch: RangedU8<31>,
    #[bits(1182..=1186)]
    pub organ_preset_1_drawbar_1_ctrl_pedal: RangedU8<31>,
    #[bits(1187..=1190)]
    pub organ_preset_1_drawbar_2: RangedU8<15>,
    #[bits(1191..=1195)]
    pub organ_preset_1_drawbar_2_wheel: RangedU8<31>,
    #[bits(1196..=1200)]
    pub organ_preset_1_drawbar_2_aftertouch: RangedU8<31>,
    #[bits(1201..=1205)]
    pub organ_preset_1_drawbar_2_ctrl_pedal: RangedU8<31>,
    #[bits(1206..=1209)]
    pub organ_preset_1_drawbar_3: RangedU8<15>,
    #[bits(1210..=1214)]
    pub organ_preset_1_drawbar_3_wheel: RangedU8<31>,
    #[bits(1215..=1219)]
    pub organ_preset_1_drawbar_3_aftertouch: RangedU8<31>,
    #[bits(1220..=1224)]
    pub organ_preset_1_drawbar_3_ctrl_pedal: RangedU8<31>,
    #[bits(1225..=1228)]
    pub organ_preset_1_drawbar_4: RangedU8<15>,
    #[bits(1229..=1233)]
    pub organ_preset_1_drawbar_4_wheel: RangedU8<31>,
    #[bits(1234..=1238)]
    pub organ_preset_1_drawbar_4_aftertouch: RangedU8<31>,
    #[bits(1239..=1243)]
    pub organ_preset_1_drawbar_4_ctrl_pedal: RangedU8<31>,
    #[bits(1244..=1247)]
    pub organ_preset_1_drawbar_5: RangedU8<15>,
    #[bits(1248..=1252)]
    pub organ_preset_1_drawbar_5_wheel: RangedU8<31>,
    #[bits(1253..=1257)]
    pub organ_preset_1_drawbar_5_aftertouch: RangedU8<31>,
    #[bits(1258..=1262)]
    pub organ_preset_1_drawbar_5_ctrl_pedal: RangedU8<31>,
    #[bits(1263..=1266)]
    pub organ_preset_1_drawbar_6: RangedU8<15>,
    #[bits(1267..=1271)]
    pub organ_preset_1_drawbar_6_wheel: RangedU8<31>,
    #[bits(1272..=1276)]
    pub organ_preset_1_drawbar_6_aftertouch: RangedU8<31>,
    #[bits(1277..=1281)]
    pub organ_preset_1_drawbar_6_ctrl_pedal: RangedU8<31>,
    #[bits(1282..=1285)]
    pub organ_preset_1_drawbar_7: RangedU8<15>,
    #[bits(1286..=1290)]
    pub organ_preset_1_drawbar_7_wheel: RangedU8<31>,
    #[bits(1291..=1295)]
    pub organ_preset_1_drawbar_7_aftertouch: RangedU8<31>,
    #[bits(1296..=1300)]
    pub organ_preset_1_drawbar_7_ctrl_pedal: RangedU8<31>,
    #[bits(1301..=1304)]
    pub organ_preset_1_drawbar_8: RangedU8<15>,
    #[bits(1305..=1309)]
    pub organ_preset_1_drawbar_8_wheel: RangedU8<31>,
    #[bits(1310..=1314)]
    pub organ_preset_1_drawbar_8_aftertouch: RangedU8<31>,
    #[bits(1315..=1319)]
    pub organ_preset_1_drawbar_8_ctrl_pedal: RangedU8<31>,
    #[bits(1320..=1323)]
    pub organ_preset_1_drawbar_9: RangedU8<15>,
    #[bits(1324..=1328)]
    pub organ_preset_1_drawbar_9_wheel: RangedU8<31>,
    #[bits(1329..=1333)]
    pub organ_preset_1_drawbar_9_aftertouch: RangedU8<31>,
    #[bits(1334..=1338)]
    pub organ_preset_1_drawbar_9_ctrl_pedal: RangedU8<31>,
    #[bits(1339..=1339)]
    pub organ_vibrato_on: bool,
    #[bits(1340..=1340)]
    pub organ_percussion_on: bool,
    #[bits(1341..=1341)]
    pub organ_percussion_harmonic_third: bool,
    #[bits(1342..=1342)]
    pub organ_percussion_decay_fast: bool,
    #[bits(1343..=1343)]
    pub organ_percussion_volume_soft: bool,
    #[bits(1384..=1387)]
    pub organ_preset_2_drawbar_1: RangedU8<15>,
    #[bits(1388..=1392)]
    pub organ_preset_2_drawbar_1_wheel: RangedU8<31>,
    #[bits(1393..=1397)]
    pub organ_preset_2_drawbar_1_aftertouch: RangedU8<31>,
    #[bits(1398..=1402)]
    pub organ_preset_2_drawbar_1_ctrl_pedal: RangedU8<31>,
    #[bits(1403..=1406)]
    pub organ_preset_2_drawbar_2: RangedU8<15>,
    #[bits(1407..=1411)]
    pub organ_preset_2_drawbar_2_wheel: RangedU8<31>,
    #[bits(1412..=1416)]
    pub organ_preset_2_drawbar_2_aftertouch: RangedU8<31>,
    #[bits(1417..=1421)]
    pub organ_preset_2_drawbar_2_ctrl_pedal: RangedU8<31>,
    #[bits(1422..=1425)]
    pub organ_preset_2_drawbar_3: RangedU8<15>,
    #[bits(1426..=1430)]
    pub organ_preset_2_drawbar_3_wheel: RangedU8<31>,
    #[bits(1431..=1435)]
    pub organ_preset_2_drawbar_3_aftertouch: RangedU8<31>,
    #[bits(1436..=1440)]
    pub organ_preset_2_drawbar_3_ctrl_pedal: RangedU8<31>,
    #[bits(1441..=1444)]
    pub organ_preset_2_drawbar_4: RangedU8<15>,
    #[bits(1445..=1449)]
    pub organ_preset_2_drawbar_4_wheel: RangedU8<31>,
    #[bits(1450..=1454)]
    pub organ_preset_2_drawbar_4_aftertouch: RangedU8<31>,
    #[bits(1455..=1459)]
    pub organ_preset_2_drawbar_4_ctrl_pedal: RangedU8<31>,
    #[bits(1460..=1463)]
    pub organ_preset_2_drawbar_5: RangedU8<15>,
    #[bits(1464..=1468)]
    pub organ_preset_2_drawbar_5_wheel: RangedU8<31>,
    #[bits(1469..=1473)]
    pub organ_preset_2_drawbar_5_aftertouch: RangedU8<31>,
    #[bits(1474..=1478)]
    pub organ_preset_2_drawbar_5_ctrl_pedal: RangedU8<31>,
    #[bits(1479..=1482)]
    pub organ_preset_2_drawbar_6: RangedU8<15>,
    #[bits(1483..=1487)]
    pub organ_preset_2_drawbar_6_wheel: RangedU8<31>,
    #[bits(1488..=1492)]
    pub organ_preset_2_drawbar_6_aftertouch: RangedU8<31>,
    #[bits(1493..=1497)]
    pub organ_preset_2_drawbar_6_ctrl_pedal: RangedU8<31>,
    #[bits(1498..=1501)]
    pub organ_preset_2_drawbar_7: RangedU8<15>,
    #[bits(1502..=1506)]
    pub organ_preset_2_drawbar_7_wheel: RangedU8<31>,
    #[bits(1507..=1511)]
    pub organ_preset_2_drawbar_7_aftertouch: RangedU8<31>,
    #[bits(1512..=1516)]
    pub organ_preset_2_drawbar_7_ctrl_pedal: RangedU8<31>,
    #[bits(1517..=1520)]
    pub organ_preset_2_drawbar_8: RangedU8<15>,
    #[bits(1521..=1525)]
    pub organ_preset_2_drawbar_8_wheel: RangedU8<31>,
    #[bits(1526..=1530)]
    pub organ_preset_2_drawbar_8_aftertouch: RangedU8<31>,
    #[bits(1531..=1535)]
    pub organ_preset_2_drawbar_8_ctrl_pedal: RangedU8<31>,
    #[bits(1536..=1539)]
    pub organ_preset_2_drawbar_9: RangedU8<15>,
    #[bits(1540..=1544)]
    pub organ_preset_2_drawbar_9_wheel: RangedU8<31>,
    #[bits(1545..=1549)]
    pub organ_preset_2_drawbar_9_aftertouch: RangedU8<31>,
    #[bits(1550..=1554)]
    pub organ_preset_2_drawbar_9_ctrl_pedal: RangedU8<31>,
    #[bits(1555..=1555)]
    pub organ_preset_2_vibrato_on: bool,
    #[bits(1556..=1556)]
    pub organ_preset_2_percussion_on: bool,
    #[bits(1557..=1557)]
    pub organ_preset_2_percussion_harmonic_third: bool,
    #[bits(1558..=1558)]
    pub organ_preset_2_percussion_decay_fast: bool,
    #[bits(1559..=1559)]
    pub organ_preset_2_percussion_volume_soft: bool,
    #[bits(1600..=1600)]
    pub extern_on: bool,
    #[bits(1601..=1603)]
    pub extern_kb_zone: RangedU8<7>,
    #[bits(1606..=1608)]
    pub extern_octave_shift: RangedU8<7>,
    #[bits(1609..=1610)]
    pub extern_midi_velocity_curve: RangedU8<3>,
    #[bits(1611..=1615)]
    pub extern_midi_channel: RangedU8<31>,
    #[bits(1616..=1616)]
    pub extern_pitch_stick: bool,
    #[bits(1617..=1617)]
    pub extern_sustain_pedal: bool,
    #[bits(1618..=1618)]
    pub extern_midi_send_wheel: bool,
    #[bits(1619..=1619)]
    pub extern_midi_send_aftertouch: bool,
    #[bits(1620..=1620)]
    pub extern_midi_send_ctrl_pedal: bool,
    #[bits(1621..=1621)]
    pub extern_midi_send_swell: bool,
    #[bits(1622..=1623)]
    pub extern_midi_control: RangedU8<3>,
    #[bits(1624..=1630)]
    pub extern_midi_cc_number: RangedU8<127>,
    #[bits(1631..=1637)]
    pub extern_midi_cc_value: RangedU8<127>,
    #[bits(1638..=1645)]
    pub extern_midi_cc_wheel: u8,
    #[bits(1646..=1653)]
    pub extern_midi_cc_aftertouch: u8,
    #[bits(1654..=1661)]
    pub extern_midi_cc_ctrl_pedal: u8,
    #[bits(1662..=1662)]
    pub extern_midi_send_user_cc_on_load: bool,
    #[bits(1663..=1670)]
    pub extern_midi_bank_select_cc32: u8,
    #[bits(1671..=1678)]
    pub extern_midi_bank_select_cc00: u8,
    #[bits(1679..=1685)]
    pub extern_midi_program: RangedU8<127>,
    #[bits(1686..=1693)]
    pub extern_midi_program_wheel: u8,
    #[bits(1694..=1701)]
    pub extern_midi_program_aftertouch: u8,
    #[bits(1702..=1709)]
    pub extern_midi_program_ctrl_pedal: u8,
    #[bits(1710..=1710)]
    pub extern_midi_send_program_on_load: bool,
    #[bits(1711..=1717)]
    pub extern_volume: RangedU8<127>,
    #[bits(1718..=1725)]
    pub extern_volume_wheel: u8,
    #[bits(1726..=1733)]
    pub extern_volume_aftertouch: u8,
    #[bits(1734..=1741)]
    pub extern_volume_ctrl_pedal: u8,
    #[bits(1742..=1742)]
    pub extern_midi_send_volume_on_load: bool,
    #[bits(1743..=1743)]
    pub extern_midi_send_volume: bool,
    #[bits(1784..=1784)]
    pub rotary_speaker_on: bool,
    #[bits(1785..=1786)]
    pub rotary_speaker_source: RangedU8<3>,
    #[bits(1787..=1787)]
    pub effect_1_on: bool,
    #[bits(1788..=1789)]
    pub effect_1_source: RangedU8<3>,
    #[bits(1790..=1792)]
    pub effect_1_type: Effect1Type,
    #[bits(1793..=1793)]
    pub effect_1_master_clock: bool,
    #[bits(1794..=1800)]
    pub effect_1_rate: RangedU8<127>,
    #[bits(1801..=1808)]
    pub effect_1_rate_wheel: u8,
    #[bits(1809..=1816)]
    pub effect_1_rate_aftertouch: u8,
    #[bits(1817..=1824)]
    pub effect_1_rate_ctrl_pedal: u8,
    #[bits(1825..=1831)]
    pub effect_1_amount: RangedU8<127>,
    #[bits(1832..=1839)]
    pub effect_1_amount_wheel: u8,
    #[bits(1840..=1847)]
    pub effect_1_amount_aftertouch: u8,
    #[bits(1848..=1855)]
    pub effect_1_amount_ctrl_pedal: u8,
    #[bits(1856..=1856)]
    pub effect_2_on: bool,
    #[bits(1857..=1858)]
    pub effect_2_source: RangedU8<3>,
    #[bits(1859..=1861)]
    pub effect_2_type: Effect2Type,
    #[bits(1862..=1868)]
    pub effect_2_rate: RangedU8<127>,
    #[bits(1869..=1875)]
    pub effect_2_amount: RangedU8<127>,
    #[bits(1876..=1883)]
    pub effect_2_amount_wheel: u8,
    #[bits(1884..=1891)]
    pub effect_2_amount_aftertouch: u8,
    #[bits(1892..=1899)]
    pub effect_2_amount_ctrl_pedal: u8,
    #[bits(1900..=1900)]
    pub delay_on: bool,
    #[bits(1901..=1902)]
    pub delay_source: RangedU8<3>,
    #[bits(1903..=1903)]
    pub delay_master_clock: bool,
    #[bits(1904..=1910)]
    pub delay_tempo: RangedU8<127>,
    #[bits(1911..=1917)]
    pub delay_tempo_lsw: RangedU8<127>,
    #[bits(1918..=1925)]
    pub delay_tempo_wheel: u8,
    #[bits(1926..=1932)]
    pub delay_tempo_wheel_lsw: RangedU8<127>,
    #[bits(1933..=1940)]
    pub delay_tempo_aftertouch: u8,
    #[bits(1941..=1947)]
    pub delay_tempo_aftertouch_lsw: RangedU8<127>,
    #[bits(1948..=1955)]
    pub delay_tempo_ctrl_pedal: u8,
    #[bits(1956..=1962)]
    pub delay_tempo_ctrl_pedal_lsw: RangedU8<127>,
    #[bits(1963..=1969)]
    pub delay_mix: RangedU8<127>,
    #[bits(1970..=1977)]
    pub delay_mix_wheel: u8,
    #[bits(1978..=1985)]
    pub delay_mix_aftertouch: u8,
    #[bits(1986..=1993)]
    pub delay_mix_ctrl_pedal: u8,
    #[bits(1994..=1994)]
    pub delay_ping_pong: bool,
    #[bits(1995..=1996)]
    pub delay_filter: RangedU8<3>,
    #[bits(1997..=2003)]
    pub delay_feedback: RangedU8<127>,
    #[bits(2004..=2011)]
    pub delay_feedback_wheel: u8,
    #[bits(2012..=2019)]
    pub delay_feedback_aftertouch: u8,
    #[bits(2020..=2027)]
    pub delay_feedback_ctrl_pedal: u8,
    #[bits(2028..=2028)]
    pub delay_analog_mode: bool,
    #[bits(2029..=2029)]
    pub amp_sim_eq_on: bool,
    #[bits(2030..=2031)]
    pub amp_sim_eq_source: RangedU8<3>,
    #[bits(2032..=2034)]
    pub amp_sim_eq_amp_type: AmpSimEqAmpType,
    #[bits(2035..=2041)]
    pub amp_sim_eq_treble: RangedU8<127>,
    #[bits(2042..=2048)]
    pub amp_sim_eq_mid_res: RangedU8<127>,
    #[bits(2049..=2055)]
    pub amp_sim_eq_bass_dry_wet: RangedU8<127>,
    #[bits(2056..=2062)]
    pub amp_sim_eq_mid_flt_freq: RangedU8<127>,
    #[bits(2063..=2070)]
    pub amp_sim_eq_mid_flt_freq_wheel: u8,
    #[bits(2071..=2078)]
    pub amp_sim_eq_mid_flt_freq_aftertouch: u8,
    #[bits(2079..=2086)]
    pub amp_sim_eq_mid_flt_freq_ctrl_pedal: u8,
    #[bits(2087..=2093)]
    pub amp_sim_eq_drive: RangedU8<127>,
    #[bits(2094..=2101)]
    pub amp_sim_eq_drive_wheel: u8,
    #[bits(2102..=2109)]
    pub amp_sim_eq_drive_aftertouch: u8,
    #[bits(2110..=2117)]
    pub amp_sim_eq_drive_ctrl_pedal: u8,
    #[bits(2118..=2118)]
    pub reverb_on: bool,
    #[bits(2119..=2121)]
    pub reverb_type: ReverbType,
    #[bits(2122..=2122)]
    pub reverb_bright: bool,
    #[bits(2123..=2129)]
    pub reverb_amount: RangedU8<127>,
    #[bits(2130..=2137)]
    pub reverb_amount_wheel: u8,
    #[bits(2138..=2145)]
    pub reverb_amount_aftertouch: u8,
    #[bits(2146..=2153)]
    pub reverb_amount_ctrl_pedal: u8,
    #[bits(2154..=2154)]
    pub compressor_on: bool,
    #[bits(2155..=2161)]
    pub compressor_amount: RangedU8<127>,
    #[bits(2162..=2162)]
    pub compressor_fast: bool,
    #[bits(2240..=2242)]
    pub program_output_main: RangedU8<7>,
    #[bits(2243..=2244)]
    pub program_output_sub_source: RangedU8<3>,
    #[bits(2245..=2246)]
    pub program_output_sub_destination: RangedU8<3>,
}

/// The preset name, 22 bytes of ASCII padded with NULs.
pub fn synth_preset_name(body: &Program) -> String {
    let raw = <[u8; BODY_LEN]>::from(body);
    String::from_utf8_lossy(&raw[44..66])
        .trim_end_matches('\0')
        .trim_end()
        .to_string()
}

/// The category byte the header's `aux` word carries; the three bytes above it
/// are zero on every corpus specimen.
pub fn category(header: &Header) -> ProgramCategory {
    use crate::bits::Packed;
    ProgramCategory::from_bits((header.aux & 0xff) as u64).expect("decoding is total")
}

/// The `(bank, location)` pair from the header, uninterpreted.
///
/// Not validated: current exports hold bank 0..=15 and location 0..=24, but v3.00
/// files in the wild hold out-of-range locations (norduserforum.com t=14414), so
/// gating on them would refuse real files.
pub fn location(file: &Cbin<Program>) -> (u16, u16) {
    file.header.slot()
}

pub fn read_from(reader: &mut (impl Read + Seek)) -> Result<Cbin<Program>, Error> {
    let file: Cbin<Program> = cbin::read(reader, FORMAT)?;
    crate::formats::known_version(FORMAT, file.header.version, KNOWN_VERSIONS)?;
    Ok(file)
}

sparse_enum!(
    /// From the `ns3-amp-sim-eq-amp-type` table in the Stage byte-map docs.
    AmpSimEqAmpType, 3, {
        0 => Clean, "Clean";
        1 => Twin, "Twin";
        2 => Jc, "JC";
        3 => Small, "Small";
        4 => Lp24, "LP24";
        5 => Hp24, "HP24";
    }
);

sparse_enum!(
    /// From the `ns3-clavinet-model` table in the Stage byte-map docs.
    ClavinetModel, 2, {
        0 => Ca, "CA";
        1 => Cb, "CB";
        2 => Da, "DA";
        3 => Db, "DB";
    }
);

sparse_enum!(
    /// From the `ns3-organ-kb-zone` table in the Stage byte-map docs.
    OrganKbZone, 4, {
        0 => V0, "o---";
        1 => V1, "-o--";
        2 => V2, "--o-";
        3 => V3, "---o";
        4 => V4, "oo--";
        5 => V5, "-oo-";
        6 => V6, "--oo";
        7 => V7, "ooo-";
        8 => V8, "-ooo";
        9 => V9, "oooo";
    }
);

sparse_enum!(
    /// From the `ns3-organ-type` table in the Stage byte-map docs.
    OrganType, 3, {
        0 => B3, "B3";
        1 => Vox, "Vox";
        2 => Farfisa, "Farfisa";
        3 => Pipe1, "Pipe1";
        4 => Pipe2, "Pipe2";
    }
);

sparse_enum!(
    /// From the `ns3-organ-vibrato-mode` table in the Stage byte-map docs.
    OrganVibratoMode, 3, {
        0 => V0, "V1";
        1 => V1, "C1";
        2 => V2, "V2";
        3 => V3, "C2";
        4 => V4, "V3";
        5 => V5, "C3";
    }
);

sparse_enum!(
    /// From the `ns3-piano-kb-touch` table in the Stage byte-map docs.
    PianoKbTouch, 2, {
        0 => Normal, "Normal";
        1 => KbTouch1, "KB Touch 1";
        2 => Touch2, "Touch 2";
        3 => Touch3, "Touch 3";
    }
);

sparse_enum!(
    /// From the `ns3-piano-layer-detune` table in the Stage byte-map docs.
    PianoLayerDetune, 2, {
        0 => V0, "Off";
        1 => V1, "1";
        2 => V2, "2";
        3 => V3, "3";
    }
);

sparse_enum!(
    /// From the `ns3-piano-timbre` table in the Stage byte-map docs.
    PianoTimbre, 3, {
        0 => None, "None";
        1 => Soft, "Soft";
        2 => Treble, "Treble";
        3 => SoftTreble, "Soft+Treble";
        4 => Brilliant, "Brilliant";
        5 => SoftBrill, "Soft+Brill";
        6 => TrebleBrill, "Treble+Brill";
        7 => SoftTrbBrill, "Soft+Trb+Brill";
    }
);

sparse_enum!(
    /// From the `ns3-piano-type` table in the Stage byte-map docs.
    PianoType, 3, {
        0 => Grand, "Grand";
        1 => Upright, "Upright";
        2 => Electric, "Electric";
        3 => Clav, "Clav";
        4 => Digital, "Digital";
        5 => Misc, "Misc";
    }
);

sparse_enum!(
    /// From the `ns3-synth-amp-env-velocity` table in the Stage byte-map docs.
    SynthAmpEnvVelocity, 2, {
        0 => V0, "Off";
        1 => V1, "1";
        2 => V2, "2";
        3 => V3, "3";
    }
);

sparse_enum!(
    /// From the `ns3-synth-arp-pattern` table in the Stage byte-map docs.
    SynthArpPattern, 2, {
        0 => Up, "Up";
        1 => Down, "Down";
        2 => UpDown, "Up/Down";
        3 => Random, "Random";
    }
);

sparse_enum!(
    /// From the `ns3-synth-arp-range` table in the Stage byte-map docs.
    SynthArpRange, 2, {
        0 => V1Octave, "1 Octave";
        1 => V2Octaves, "2 Octaves";
        2 => V3Octaves, "3 Octaves";
        3 => V4Octaves, "4 Octaves";
    }
);

sparse_enum!(
    /// From the `ns3-synth-filter-drive` table in the Stage byte-map docs.
    SynthFilterDrive, 2, {
        0 => V0, "Off";
        1 => V1, "1";
        2 => V2, "2";
        3 => V3, "3";
    }
);

sparse_enum!(
    /// From the `ns3-synth-filter-kb-track` table in the Stage byte-map docs.
    SynthFilterKbTrack, 2, {
        0 => V0, "Off";
        1 => V1, "1/3";
        2 => V2, "2/3";
        3 => V3, "1";
    }
);

sparse_enum!(
    /// From the `ns3-synth-filter-type` table in the Stage byte-map docs.
    SynthFilterType, 3, {
        0 => Lp12, "LP12";
        1 => Lp24, "LP24";
        2 => MiniMoog, "Mini Moog";
        3 => LpHp, "LP+HP";
        4 => Bp24, "BP24";
        5 => Hp24, "HP24";
    }
);

sparse_enum!(
    /// From the `ns3-synth-lfo-wave` table in the Stage byte-map docs.
    SynthLfoWave, 3, {
        0 => Triangle, "Triangle";
        1 => Saw, "Saw";
        2 => NegSaw, "Neg Saw";
        3 => Square, "Square";
        4 => SH, "S/H";
    }
);

sparse_enum!(
    /// From the `ns3-synth-oscillator-config` table in the Stage byte-map docs.
    SynthOscillatorConfig, 4, {
        0 => None, "None";
        1 => Pitch, "Pitch";
        2 => Shape, "Shape";
        3 => Sync, "Sync";
        4 => Detune, "Detune";
        5 => Mixsin, "MixSin";
        6 => Mixtri, "MixTri";
        7 => Mixsaw, "MixSaw";
        8 => Mixsqr, "MixSqr";
        9 => Mixbell, "MixBell";
        10 => Mixns1, "MixNs1";
        11 => Mixns2, "MixNs2";
        12 => Fm1, "FM1";
        13 => Fm2, "FM2";
        14 => Rm, "RM";
    }
);

sparse_enum!(
    /// From the `ns3-synth-oscillator-type` table in the Stage byte-map docs.
    SynthOscillatorType, 3, {
        0 => Classic, "Classic";
        1 => Wave, "Wave";
        2 => Formant, "Formant";
        3 => Super, "Super";
        4 => Sample, "Sample";
    }
);

sparse_enum!(
    /// From the `ns3-synth-unison` table in the Stage byte-map docs.
    SynthUnison, 2, {
        0 => V0, "Off";
        1 => V1, "1";
        2 => V2, "2";
        3 => V3, "3";
    }
);

sparse_enum!(
    /// From the `ns3-synth-vibrato` table in the Stage byte-map docs.
    SynthVibrato, 3, {
        0 => Off, "Off";
        1 => Delay1, "Delay 1";
        2 => Delay2, "Delay 2";
        3 => Delay3, "Delay 3";
        4 => Wheel, "Wheel";
        5 => AfterTouch, "After Touch";
    }
);

sparse_enum!(
    /// From the `ns3-synth-voice` table in the Stage byte-map docs.
    SynthVoice, 2, {
        0 => Poly, "Poly";
        1 => Legato, "Legato";
        2 => Mono, "Mono";
    }
);
