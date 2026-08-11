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
    // ── The second panel. The byte maps document one; the body holds two,
    //    the second repeating the first 263 bytes later.
    #[bits(2288..=2288)]
    pub panel_b_piano_on: bool,
    #[bits(2289..=2292)]
    pub panel_b_piano_kb_zone: RangedU8<15>,
    #[bits(2293..=2299)]
    pub panel_b_piano_volume: RangedU8<127>,
    #[bits(2300..=2307)]
    pub panel_b_piano_volume_wheel: u8,
    #[bits(2308..=2315)]
    pub panel_b_piano_volume_aftertouch: u8,
    #[bits(2316..=2323)]
    pub panel_b_piano_volume_ctrl_pedal: u8,
    #[bits(2324..=2327)]
    pub panel_b_piano_octave_shift: RangedU8<15>,
    #[bits(2328..=2328)]
    pub panel_b_piano_pitch_stick: bool,
    #[bits(2329..=2329)]
    pub panel_b_piano_sustain_pedal: bool,
    #[bits(2330..=2332)]
    pub panel_b_piano_type: PianoType,
    #[bits(2333..=2337)]
    pub panel_b_piano_model: RangedU8<31>,
    #[bits(2338..=2339)]
    pub panel_b_clavinet_model: ClavinetModel,
    #[bits(2340..=2371)]
    pub panel_b_piano_sample_name: u32,
    #[bits(2372..=2372)]
    pub panel_b_piano_soft_release: bool,
    #[bits(2373..=2373)]
    pub panel_b_piano_string_resonance: bool,
    #[bits(2374..=2374)]
    pub panel_b_piano_pedal_noise: bool,
    #[bits(2375..=2376)]
    pub panel_b_piano_kb_touch: PianoKbTouch,
    #[bits(2378..=2380)]
    pub panel_b_piano_timbre: PianoTimbre,
    #[bits(2408..=2408)]
    pub panel_b_synth_on: bool,
    #[bits(2409..=2412)]
    pub panel_b_synth_kb_zone: RangedU8<15>,
    #[bits(2413..=2419)]
    pub panel_b_synth_volume: RangedU8<127>,
    #[bits(2420..=2427)]
    pub panel_b_synth_volume_wheel: u8,
    #[bits(2428..=2435)]
    pub panel_b_synth_volume_aftertouch: u8,
    #[bits(2436..=2443)]
    pub panel_b_synth_volume_ctrl_pedal: u8,
    #[bits(2444..=2447)]
    pub panel_b_synth_octave_shift: RangedU8<15>,
    #[bits(2448..=2448)]
    pub panel_b_synth_pitch_stick: bool,
    #[bits(2449..=2449)]
    pub panel_b_synth_sustain_pedal: bool,
    #[bits(2450..=2459)]
    pub panel_b_synth_preset_location: RangedU16<1023>,
    #[bits(2776..=2776)]
    pub panel_b_synth_kh_hold: bool,
    #[bits(2777..=2777)]
    pub panel_b_synth_arp_on: bool,
    #[bits(2778..=2778)]
    pub panel_b_synth_arp_kb_sync: bool,
    #[bits(2779..=2780)]
    pub panel_b_synth_arp_range: SynthArpRange,
    #[bits(2781..=2782)]
    pub panel_b_synth_arp_pattern: SynthArpPattern,
    #[bits(2783..=2783)]
    pub panel_b_synth_arp_master_clock: bool,
    #[bits(2784..=2790)]
    pub panel_b_synth_arp_rate: RangedU8<127>,
    #[bits(2791..=2798)]
    pub panel_b_synth_arp_rate_wheel: u8,
    #[bits(2799..=2806)]
    pub panel_b_synth_arp_rate_aftertouch: u8,
    #[bits(2807..=2814)]
    pub panel_b_synth_arp_rate_ctrl_pedal: u8,
    #[bits(2815..=2816)]
    pub panel_b_synth_voice: SynthVoice,
    #[bits(2817..=2823)]
    pub panel_b_synth_glide: RangedU8<127>,
    #[bits(2824..=2825)]
    pub panel_b_synth_unison: SynthUnison,
    #[bits(2826..=2828)]
    pub panel_b_synth_vibrato: SynthVibrato,
    #[bits(2829..=2831)]
    pub panel_b_synth_lfo_wave: SynthLfoWave,
    #[bits(2832..=2832)]
    pub panel_b_synth_lfo_master_clock: bool,
    #[bits(2833..=2839)]
    pub panel_b_synth_lfo_rate: RangedU8<127>,
    #[bits(2840..=2847)]
    pub panel_b_synth_lfo_rate_wheel: u8,
    #[bits(2848..=2855)]
    pub panel_b_synth_lfo_rate_aftertouch: u8,
    #[bits(2856..=2863)]
    pub panel_b_synth_lfo_rate_ctrl_pedal: u8,
    #[bits(2864..=2870)]
    pub panel_b_synth_mod_env_attack: RangedU8<127>,
    #[bits(2871..=2877)]
    pub panel_b_synth_mod_env_decay: RangedU8<127>,
    #[bits(2878..=2884)]
    pub panel_b_synth_mod_env_release: RangedU8<127>,
    #[bits(2885..=2885)]
    pub panel_b_synth_mod_env_velocity: bool,
    #[bits(2886..=2888)]
    pub panel_b_synth_oscillator_type: SynthOscillatorType,
    #[bits(2889..=2897)]
    pub panel_b_synth_oscillator_1_wave_form: RangedU16<511>,
    #[bits(2899..=2902)]
    pub panel_b_synth_oscillator_config: SynthOscillatorConfig,
    #[bits(2903..=2908)]
    pub panel_b_synth_pitch: RangedU8<63>,
    #[bits(2909..=2915)]
    pub panel_b_synth_oscillator_control: RangedU8<127>,
    #[bits(2916..=2923)]
    pub panel_b_synth_oscillator_control_wheel: u8,
    #[bits(2924..=2931)]
    pub panel_b_synth_oscillator_control_aftertouch: u8,
    #[bits(2932..=2939)]
    pub panel_b_synth_oscillator_control_ctrl_pedal: u8,
    #[bits(2940..=2946)]
    pub panel_b_synth_oscillator_mod: RangedU8<127>,
    #[bits(2947..=2954)]
    pub panel_b_synth_oscillator_mod_wheel: u8,
    #[bits(2955..=2962)]
    pub panel_b_synth_oscillator_mod_aftertouch: u8,
    #[bits(2963..=2970)]
    pub panel_b_synth_oscillator_mod_ctrl_pedal: u8,
    #[bits(2971..=2973)]
    pub panel_b_synth_filter_type: SynthFilterType,
    #[bits(2974..=2980)]
    pub panel_b_synth_filter_freq: RangedU8<127>,
    #[bits(2981..=2988)]
    pub panel_b_synth_filter_freq_wheel: u8,
    #[bits(2989..=2996)]
    pub panel_b_synth_filter_freq_aftertouch: u8,
    #[bits(2997..=3004)]
    pub panel_b_synth_filter_freq_ctrl_pedal: u8,
    #[bits(3005..=3011)]
    pub panel_b_synth_filter_hp_freq_res: RangedU8<127>,
    #[bits(3012..=3019)]
    pub panel_b_synth_filter_hp_freq_res_wheel: u8,
    #[bits(3020..=3027)]
    pub panel_b_synth_filter_hp_freq_res_aftertouch: u8,
    #[bits(3028..=3035)]
    pub panel_b_synth_filter_hp_freq_res_ctrl_pedal: u8,
    #[bits(3036..=3042)]
    pub panel_b_synth_filter_lfo_amount: RangedU8<127>,
    #[bits(3043..=3050)]
    pub panel_b_synth_filter_lfo_amount_wheel: u8,
    #[bits(3051..=3058)]
    pub panel_b_synth_filter_lfo_amount_aftertouch: u8,
    #[bits(3059..=3066)]
    pub panel_b_synth_filter_lfo_amount_ctrl_pedal: u8,
    #[bits(3067..=3073)]
    pub panel_b_synth_filter_vel_mod_env_amount: RangedU8<127>,
    #[bits(3074..=3075)]
    pub panel_b_synth_filter_kb_track: SynthFilterKbTrack,
    #[bits(3076..=3077)]
    pub panel_b_synth_filter_drive: SynthFilterDrive,
    #[bits(3078..=3084)]
    pub panel_b_synth_amp_env_attack: RangedU8<127>,
    #[bits(3085..=3091)]
    pub panel_b_synth_amp_env_decay: RangedU8<127>,
    #[bits(3092..=3098)]
    pub panel_b_synth_amp_env_release: RangedU8<127>,
    #[bits(3099..=3100)]
    pub panel_b_synth_amp_env_velocity: SynthAmpEnvVelocity,
    #[bits(3101..=3132)]
    pub panel_b_synth_sample_id: u32,
    #[bits(3133..=3133)]
    pub panel_b_synth_fast_attack: bool,
    #[bits(3208..=3208)]
    pub panel_b_organ_on: bool,
    #[bits(3209..=3212)]
    pub panel_b_organ_kb_zone: OrganKbZone,
    #[bits(3213..=3219)]
    pub panel_b_organ_volume: RangedU8<127>,
    #[bits(3220..=3227)]
    pub panel_b_organ_volume_wheel: u8,
    #[bits(3228..=3235)]
    pub panel_b_organ_volume_aftertouch: u8,
    #[bits(3236..=3243)]
    pub panel_b_organ_volume_ctrl_pedal: u8,
    #[bits(3244..=3247)]
    pub panel_b_organ_octave_shift: RangedU8<15>,
    #[bits(3248..=3248)]
    pub panel_b_organ_sustain_pedal: bool,
    #[bits(3249..=3251)]
    pub panel_b_organ_type: OrganType,
    #[bits(3252..=3252)]
    pub panel_b_organ_live_mode: bool,
    #[bits(3253..=3253)]
    pub panel_b_organ_preset_2_on: bool,
    #[bits(3272..=3275)]
    pub panel_b_organ_preset_1_drawbar_1: RangedU8<15>,
    #[bits(3276..=3280)]
    pub panel_b_organ_preset_1_drawbar_1_wheel: RangedU8<31>,
    #[bits(3281..=3285)]
    pub panel_b_organ_preset_1_drawbar_1_aftertouch: RangedU8<31>,
    #[bits(3286..=3290)]
    pub panel_b_organ_preset_1_drawbar_1_ctrl_pedal: RangedU8<31>,
    #[bits(3291..=3294)]
    pub panel_b_organ_preset_1_drawbar_2: RangedU8<15>,
    #[bits(3295..=3299)]
    pub panel_b_organ_preset_1_drawbar_2_wheel: RangedU8<31>,
    #[bits(3300..=3304)]
    pub panel_b_organ_preset_1_drawbar_2_aftertouch: RangedU8<31>,
    #[bits(3305..=3309)]
    pub panel_b_organ_preset_1_drawbar_2_ctrl_pedal: RangedU8<31>,
    #[bits(3310..=3313)]
    pub panel_b_organ_preset_1_drawbar_3: RangedU8<15>,
    #[bits(3314..=3318)]
    pub panel_b_organ_preset_1_drawbar_3_wheel: RangedU8<31>,
    #[bits(3319..=3323)]
    pub panel_b_organ_preset_1_drawbar_3_aftertouch: RangedU8<31>,
    #[bits(3324..=3328)]
    pub panel_b_organ_preset_1_drawbar_3_ctrl_pedal: RangedU8<31>,
    #[bits(3329..=3332)]
    pub panel_b_organ_preset_1_drawbar_4: RangedU8<15>,
    #[bits(3333..=3337)]
    pub panel_b_organ_preset_1_drawbar_4_wheel: RangedU8<31>,
    #[bits(3338..=3342)]
    pub panel_b_organ_preset_1_drawbar_4_aftertouch: RangedU8<31>,
    #[bits(3343..=3347)]
    pub panel_b_organ_preset_1_drawbar_4_ctrl_pedal: RangedU8<31>,
    #[bits(3348..=3351)]
    pub panel_b_organ_preset_1_drawbar_5: RangedU8<15>,
    #[bits(3352..=3356)]
    pub panel_b_organ_preset_1_drawbar_5_wheel: RangedU8<31>,
    #[bits(3357..=3361)]
    pub panel_b_organ_preset_1_drawbar_5_aftertouch: RangedU8<31>,
    #[bits(3362..=3366)]
    pub panel_b_organ_preset_1_drawbar_5_ctrl_pedal: RangedU8<31>,
    #[bits(3367..=3370)]
    pub panel_b_organ_preset_1_drawbar_6: RangedU8<15>,
    #[bits(3371..=3375)]
    pub panel_b_organ_preset_1_drawbar_6_wheel: RangedU8<31>,
    #[bits(3376..=3380)]
    pub panel_b_organ_preset_1_drawbar_6_aftertouch: RangedU8<31>,
    #[bits(3381..=3385)]
    pub panel_b_organ_preset_1_drawbar_6_ctrl_pedal: RangedU8<31>,
    #[bits(3386..=3389)]
    pub panel_b_organ_preset_1_drawbar_7: RangedU8<15>,
    #[bits(3390..=3394)]
    pub panel_b_organ_preset_1_drawbar_7_wheel: RangedU8<31>,
    #[bits(3395..=3399)]
    pub panel_b_organ_preset_1_drawbar_7_aftertouch: RangedU8<31>,
    #[bits(3400..=3404)]
    pub panel_b_organ_preset_1_drawbar_7_ctrl_pedal: RangedU8<31>,
    #[bits(3405..=3408)]
    pub panel_b_organ_preset_1_drawbar_8: RangedU8<15>,
    #[bits(3409..=3413)]
    pub panel_b_organ_preset_1_drawbar_8_wheel: RangedU8<31>,
    #[bits(3414..=3418)]
    pub panel_b_organ_preset_1_drawbar_8_aftertouch: RangedU8<31>,
    #[bits(3419..=3423)]
    pub panel_b_organ_preset_1_drawbar_8_ctrl_pedal: RangedU8<31>,
    #[bits(3424..=3427)]
    pub panel_b_organ_preset_1_drawbar_9: RangedU8<15>,
    #[bits(3428..=3432)]
    pub panel_b_organ_preset_1_drawbar_9_wheel: RangedU8<31>,
    #[bits(3433..=3437)]
    pub panel_b_organ_preset_1_drawbar_9_aftertouch: RangedU8<31>,
    #[bits(3438..=3442)]
    pub panel_b_organ_preset_1_drawbar_9_ctrl_pedal: RangedU8<31>,
    #[bits(3443..=3443)]
    pub panel_b_organ_vibrato_on: bool,
    #[bits(3444..=3444)]
    pub panel_b_organ_percussion_on: bool,
    #[bits(3445..=3445)]
    pub panel_b_organ_percussion_harmonic_third: bool,
    #[bits(3446..=3446)]
    pub panel_b_organ_percussion_decay_fast: bool,
    #[bits(3447..=3447)]
    pub panel_b_organ_percussion_volume_soft: bool,
    #[bits(3488..=3491)]
    pub panel_b_organ_preset_2_drawbar_1: RangedU8<15>,
    #[bits(3492..=3496)]
    pub panel_b_organ_preset_2_drawbar_1_wheel: RangedU8<31>,
    #[bits(3497..=3501)]
    pub panel_b_organ_preset_2_drawbar_1_aftertouch: RangedU8<31>,
    #[bits(3502..=3506)]
    pub panel_b_organ_preset_2_drawbar_1_ctrl_pedal: RangedU8<31>,
    #[bits(3507..=3510)]
    pub panel_b_organ_preset_2_drawbar_2: RangedU8<15>,
    #[bits(3511..=3515)]
    pub panel_b_organ_preset_2_drawbar_2_wheel: RangedU8<31>,
    #[bits(3516..=3520)]
    pub panel_b_organ_preset_2_drawbar_2_aftertouch: RangedU8<31>,
    #[bits(3521..=3525)]
    pub panel_b_organ_preset_2_drawbar_2_ctrl_pedal: RangedU8<31>,
    #[bits(3526..=3529)]
    pub panel_b_organ_preset_2_drawbar_3: RangedU8<15>,
    #[bits(3530..=3534)]
    pub panel_b_organ_preset_2_drawbar_3_wheel: RangedU8<31>,
    #[bits(3535..=3539)]
    pub panel_b_organ_preset_2_drawbar_3_aftertouch: RangedU8<31>,
    #[bits(3540..=3544)]
    pub panel_b_organ_preset_2_drawbar_3_ctrl_pedal: RangedU8<31>,
    #[bits(3545..=3548)]
    pub panel_b_organ_preset_2_drawbar_4: RangedU8<15>,
    #[bits(3549..=3553)]
    pub panel_b_organ_preset_2_drawbar_4_wheel: RangedU8<31>,
    #[bits(3554..=3558)]
    pub panel_b_organ_preset_2_drawbar_4_aftertouch: RangedU8<31>,
    #[bits(3559..=3563)]
    pub panel_b_organ_preset_2_drawbar_4_ctrl_pedal: RangedU8<31>,
    #[bits(3564..=3567)]
    pub panel_b_organ_preset_2_drawbar_5: RangedU8<15>,
    #[bits(3568..=3572)]
    pub panel_b_organ_preset_2_drawbar_5_wheel: RangedU8<31>,
    #[bits(3573..=3577)]
    pub panel_b_organ_preset_2_drawbar_5_aftertouch: RangedU8<31>,
    #[bits(3578..=3582)]
    pub panel_b_organ_preset_2_drawbar_5_ctrl_pedal: RangedU8<31>,
    #[bits(3583..=3586)]
    pub panel_b_organ_preset_2_drawbar_6: RangedU8<15>,
    #[bits(3587..=3591)]
    pub panel_b_organ_preset_2_drawbar_6_wheel: RangedU8<31>,
    #[bits(3592..=3596)]
    pub panel_b_organ_preset_2_drawbar_6_aftertouch: RangedU8<31>,
    #[bits(3597..=3601)]
    pub panel_b_organ_preset_2_drawbar_6_ctrl_pedal: RangedU8<31>,
    #[bits(3602..=3605)]
    pub panel_b_organ_preset_2_drawbar_7: RangedU8<15>,
    #[bits(3606..=3610)]
    pub panel_b_organ_preset_2_drawbar_7_wheel: RangedU8<31>,
    #[bits(3611..=3615)]
    pub panel_b_organ_preset_2_drawbar_7_aftertouch: RangedU8<31>,
    #[bits(3616..=3620)]
    pub panel_b_organ_preset_2_drawbar_7_ctrl_pedal: RangedU8<31>,
    #[bits(3621..=3624)]
    pub panel_b_organ_preset_2_drawbar_8: RangedU8<15>,
    #[bits(3625..=3629)]
    pub panel_b_organ_preset_2_drawbar_8_wheel: RangedU8<31>,
    #[bits(3630..=3634)]
    pub panel_b_organ_preset_2_drawbar_8_aftertouch: RangedU8<31>,
    #[bits(3635..=3639)]
    pub panel_b_organ_preset_2_drawbar_8_ctrl_pedal: RangedU8<31>,
    #[bits(3640..=3643)]
    pub panel_b_organ_preset_2_drawbar_9: RangedU8<15>,
    #[bits(3644..=3648)]
    pub panel_b_organ_preset_2_drawbar_9_wheel: RangedU8<31>,
    #[bits(3649..=3653)]
    pub panel_b_organ_preset_2_drawbar_9_aftertouch: RangedU8<31>,
    #[bits(3654..=3658)]
    pub panel_b_organ_preset_2_drawbar_9_ctrl_pedal: RangedU8<31>,
    #[bits(3659..=3659)]
    pub panel_b_organ_preset_2_vibrato_on: bool,
    #[bits(3660..=3660)]
    pub panel_b_organ_preset_2_percussion_on: bool,
    #[bits(3661..=3661)]
    pub panel_b_organ_preset_2_percussion_harmonic_third: bool,
    #[bits(3662..=3662)]
    pub panel_b_organ_preset_2_percussion_decay_fast: bool,
    #[bits(3663..=3663)]
    pub panel_b_organ_preset_2_percussion_volume_soft: bool,
    #[bits(3704..=3704)]
    pub panel_b_extern_on: bool,
    #[bits(3705..=3707)]
    pub panel_b_extern_kb_zone: RangedU8<7>,
    #[bits(3710..=3712)]
    pub panel_b_extern_octave_shift: RangedU8<7>,
    #[bits(3713..=3714)]
    pub panel_b_extern_midi_velocity_curve: RangedU8<3>,
    #[bits(3715..=3719)]
    pub panel_b_extern_midi_channel: RangedU8<31>,
    #[bits(3720..=3720)]
    pub panel_b_extern_pitch_stick: bool,
    #[bits(3721..=3721)]
    pub panel_b_extern_sustain_pedal: bool,
    #[bits(3722..=3722)]
    pub panel_b_extern_midi_send_wheel: bool,
    #[bits(3723..=3723)]
    pub panel_b_extern_midi_send_aftertouch: bool,
    #[bits(3724..=3724)]
    pub panel_b_extern_midi_send_ctrl_pedal: bool,
    #[bits(3725..=3725)]
    pub panel_b_extern_midi_send_swell: bool,
    #[bits(3726..=3727)]
    pub panel_b_extern_midi_control: RangedU8<3>,
    #[bits(3728..=3734)]
    pub panel_b_extern_midi_cc_number: RangedU8<127>,
    #[bits(3735..=3741)]
    pub panel_b_extern_midi_cc_value: RangedU8<127>,
    #[bits(3742..=3749)]
    pub panel_b_extern_midi_cc_wheel: u8,
    #[bits(3750..=3757)]
    pub panel_b_extern_midi_cc_aftertouch: u8,
    #[bits(3758..=3765)]
    pub panel_b_extern_midi_cc_ctrl_pedal: u8,
    #[bits(3766..=3766)]
    pub panel_b_extern_midi_send_user_cc_on_load: bool,
    #[bits(3767..=3774)]
    pub panel_b_extern_midi_bank_select_cc32: u8,
    #[bits(3775..=3782)]
    pub panel_b_extern_midi_bank_select_cc00: u8,
    #[bits(3783..=3789)]
    pub panel_b_extern_midi_program: RangedU8<127>,
    #[bits(3790..=3797)]
    pub panel_b_extern_midi_program_wheel: u8,
    #[bits(3798..=3805)]
    pub panel_b_extern_midi_program_aftertouch: u8,
    #[bits(3806..=3813)]
    pub panel_b_extern_midi_program_ctrl_pedal: u8,
    #[bits(3814..=3814)]
    pub panel_b_extern_midi_send_program_on_load: bool,
    #[bits(3815..=3821)]
    pub panel_b_extern_volume: RangedU8<127>,
    #[bits(3822..=3829)]
    pub panel_b_extern_volume_wheel: u8,
    #[bits(3830..=3837)]
    pub panel_b_extern_volume_aftertouch: u8,
    #[bits(3838..=3845)]
    pub panel_b_extern_volume_ctrl_pedal: u8,
    #[bits(3846..=3846)]
    pub panel_b_extern_midi_send_volume_on_load: bool,
    #[bits(3847..=3847)]
    pub panel_b_extern_midi_send_volume: bool,
    #[bits(3888..=3888)]
    pub panel_b_rotary_speaker_on: bool,
    #[bits(3889..=3890)]
    pub panel_b_rotary_speaker_source: RangedU8<3>,
    #[bits(3891..=3891)]
    pub panel_b_effect_1_on: bool,
    #[bits(3892..=3893)]
    pub panel_b_effect_1_source: RangedU8<3>,
    #[bits(3894..=3896)]
    pub panel_b_effect_1_type: Effect1Type,
    #[bits(3897..=3897)]
    pub panel_b_effect_1_master_clock: bool,
    #[bits(3898..=3904)]
    pub panel_b_effect_1_rate: RangedU8<127>,
    #[bits(3905..=3912)]
    pub panel_b_effect_1_rate_wheel: u8,
    #[bits(3913..=3920)]
    pub panel_b_effect_1_rate_aftertouch: u8,
    #[bits(3921..=3928)]
    pub panel_b_effect_1_rate_ctrl_pedal: u8,
    #[bits(3929..=3935)]
    pub panel_b_effect_1_amount: RangedU8<127>,
    #[bits(3936..=3943)]
    pub panel_b_effect_1_amount_wheel: u8,
    #[bits(3944..=3951)]
    pub panel_b_effect_1_amount_aftertouch: u8,
    #[bits(3952..=3959)]
    pub panel_b_effect_1_amount_ctrl_pedal: u8,
    #[bits(3960..=3960)]
    pub panel_b_effect_2_on: bool,
    #[bits(3961..=3962)]
    pub panel_b_effect_2_source: RangedU8<3>,
    #[bits(3963..=3965)]
    pub panel_b_effect_2_type: Effect2Type,
    #[bits(3966..=3972)]
    pub panel_b_effect_2_rate: RangedU8<127>,
    #[bits(3973..=3979)]
    pub panel_b_effect_2_amount: RangedU8<127>,
    #[bits(3980..=3987)]
    pub panel_b_effect_2_amount_wheel: u8,
    #[bits(3988..=3995)]
    pub panel_b_effect_2_amount_aftertouch: u8,
    #[bits(3996..=4003)]
    pub panel_b_effect_2_amount_ctrl_pedal: u8,
    #[bits(4004..=4004)]
    pub panel_b_delay_on: bool,
    #[bits(4005..=4006)]
    pub panel_b_delay_source: RangedU8<3>,
    #[bits(4007..=4007)]
    pub panel_b_delay_master_clock: bool,
    #[bits(4008..=4014)]
    pub panel_b_delay_tempo: RangedU8<127>,
    #[bits(4015..=4021)]
    pub panel_b_delay_tempo_lsw: RangedU8<127>,
    #[bits(4022..=4029)]
    pub panel_b_delay_tempo_wheel: u8,
    #[bits(4030..=4036)]
    pub panel_b_delay_tempo_wheel_lsw: RangedU8<127>,
    #[bits(4037..=4044)]
    pub panel_b_delay_tempo_aftertouch: u8,
    #[bits(4045..=4051)]
    pub panel_b_delay_tempo_aftertouch_lsw: RangedU8<127>,
    #[bits(4052..=4059)]
    pub panel_b_delay_tempo_ctrl_pedal: u8,
    #[bits(4060..=4066)]
    pub panel_b_delay_tempo_ctrl_pedal_lsw: RangedU8<127>,
    #[bits(4067..=4073)]
    pub panel_b_delay_mix: RangedU8<127>,
    #[bits(4074..=4081)]
    pub panel_b_delay_mix_wheel: u8,
    #[bits(4082..=4089)]
    pub panel_b_delay_mix_aftertouch: u8,
    #[bits(4090..=4097)]
    pub panel_b_delay_mix_ctrl_pedal: u8,
    #[bits(4098..=4098)]
    pub panel_b_delay_ping_pong: bool,
    #[bits(4099..=4100)]
    pub panel_b_delay_filter: RangedU8<3>,
    #[bits(4101..=4107)]
    pub panel_b_delay_feedback: RangedU8<127>,
    #[bits(4108..=4115)]
    pub panel_b_delay_feedback_wheel: u8,
    #[bits(4116..=4123)]
    pub panel_b_delay_feedback_aftertouch: u8,
    #[bits(4124..=4131)]
    pub panel_b_delay_feedback_ctrl_pedal: u8,
    #[bits(4132..=4132)]
    pub panel_b_delay_analog_mode: bool,
    #[bits(4133..=4133)]
    pub panel_b_amp_sim_eq_on: bool,
    #[bits(4134..=4135)]
    pub panel_b_amp_sim_eq_source: RangedU8<3>,
    #[bits(4136..=4138)]
    pub panel_b_amp_sim_eq_amp_type: AmpSimEqAmpType,
    #[bits(4139..=4145)]
    pub panel_b_amp_sim_eq_treble: RangedU8<127>,
    #[bits(4146..=4152)]
    pub panel_b_amp_sim_eq_mid_res: RangedU8<127>,
    #[bits(4153..=4159)]
    pub panel_b_amp_sim_eq_bass_dry_wet: RangedU8<127>,
    #[bits(4160..=4166)]
    pub panel_b_amp_sim_eq_mid_flt_freq: RangedU8<127>,
    #[bits(4167..=4174)]
    pub panel_b_amp_sim_eq_mid_flt_freq_wheel: u8,
    #[bits(4175..=4182)]
    pub panel_b_amp_sim_eq_mid_flt_freq_aftertouch: u8,
    #[bits(4183..=4190)]
    pub panel_b_amp_sim_eq_mid_flt_freq_ctrl_pedal: u8,
    #[bits(4191..=4197)]
    pub panel_b_amp_sim_eq_drive: RangedU8<127>,
    #[bits(4198..=4205)]
    pub panel_b_amp_sim_eq_drive_wheel: u8,
    #[bits(4206..=4213)]
    pub panel_b_amp_sim_eq_drive_aftertouch: u8,
    #[bits(4214..=4221)]
    pub panel_b_amp_sim_eq_drive_ctrl_pedal: u8,
    #[bits(4222..=4222)]
    pub panel_b_reverb_on: bool,
    #[bits(4223..=4225)]
    pub panel_b_reverb_type: ReverbType,
    #[bits(4226..=4226)]
    pub panel_b_reverb_bright: bool,
    #[bits(4227..=4233)]
    pub panel_b_reverb_amount: RangedU8<127>,
    #[bits(4234..=4241)]
    pub panel_b_reverb_amount_wheel: u8,
    #[bits(4242..=4249)]
    pub panel_b_reverb_amount_aftertouch: u8,
    #[bits(4250..=4257)]
    pub panel_b_reverb_amount_ctrl_pedal: u8,
    #[bits(4258..=4258)]
    pub panel_b_compressor_on: bool,
    #[bits(4259..=4265)]
    pub panel_b_compressor_amount: RangedU8<127>,
    #[bits(4266..=4266)]
    pub panel_b_compressor_fast: bool,
    #[bits(4344..=4346)]
    pub panel_b_program_output_main: RangedU8<7>,
    #[bits(4347..=4348)]
    pub panel_b_program_output_sub_source: RangedU8<3>,
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
