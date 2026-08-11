//! The Stage 4 synth preset body (`.ns4y`): 497 bytes, 504 parameters.
//!
//! One synth section as a program stores it, moved down 327 bytes, without the
//! two placements a preset has no use for — the keyboard zone, which belongs to
//! the program that loads it, and the extern CC values.

use crate::cbin::{self, Cbin};
use crate::error::Error;
use crate::types::{RangedU16, RangedU8};
use std::io::{Read, Seek};

pub const FORMAT: &str = "ns4y";
/// Stored ×100. The corpus holds 2.08; ns4decode was tested from 2.03.
pub const KNOWN_VERSIONS: &[u32] = &[203, 204, 205, 206, 207, 208];
pub const BODY_LEN: usize = 497;

#[nord_bits_derive::bitbody(497)]
pub struct SynthPreset {
    #[bits(42..=42)]
    pub synth_c_layer_enabled: bool,
    #[bits(43..=43)]
    pub synth_b_layer_enabled: bool,
    #[bits(44..=44)]
    pub synth_a_layer_enabled: bool,
    #[bits(47..=47)]
    pub synth_c_layer_enabled_scene_2: bool,
    #[bits(48..=48)]
    pub synth_b_layer_enabled_scene_2: bool,
    #[bits(49..=49)]
    pub synth_a_layer_enabled_scene_2: bool,
    #[bits(50..=56)]
    pub synth_a_volume: RangedU8<127>,
    #[bits(57..=64)]
    pub synth_a_volume_wheel: u8,
    #[bits(65..=72)]
    pub synth_a_volume_aftertouch: u8,
    #[bits(73..=80)]
    pub synth_a_volume_ctrl_pedal: u8,
    #[bits(81..=87)]
    pub synth_b_volume: RangedU8<127>,
    #[bits(88..=95)]
    pub synth_b_volume_wheel: u8,
    #[bits(96..=103)]
    pub synth_b_volume_aftertouch: u8,
    #[bits(104..=111)]
    pub synth_b_volume_ctrl_pedal: u8,
    #[bits(112..=118)]
    pub synth_c_volume: RangedU8<127>,
    #[bits(119..=126)]
    pub synth_c_volume_wheel: u8,
    #[bits(127..=134)]
    pub synth_c_volume_aftertouch: u8,
    #[bits(135..=142)]
    pub synth_c_volume_ctrl_pedal: u8,
    #[bits(143..=148)]
    pub synth_a_pan: RangedU8<63>,
    #[bits(174..=179)]
    pub synth_b_pan: RangedU8<63>,
    #[bits(205..=210)]
    pub synth_c_pan: RangedU8<63>,
    #[bits(288..=288)]
    pub synth_a_samples_analog: bool,
    #[bits(294..=305)]
    pub synth_a_sample_slot: RangedU16<4095>,
    #[bits(306..=337)]
    pub synth_a_sample_id: u32,
    #[bits(342..=345)]
    pub synth_a_octave_shift: RangedU8<15>,
    #[bits(346..=346)]
    pub synth_a_pitch_stick_enabled: bool,
    #[bits(347..=350)]
    pub synth_a_pitch_stick_range: RangedU8<15>,
    #[bits(351..=351)]
    pub synth_a_sustain_pedal_enabled: bool,
    #[bits(352..=354)]
    pub synth_a_vibrato_mode: RangedU8<7>,
    #[bits(357..=357)]
    pub synth_a_legato_enabled: bool,
    #[bits(358..=358)]
    pub synth_a_mono_enabled: bool,
    #[bits(359..=360)]
    pub synth_a_voice_priority: RangedU8<3>,
    #[bits(361..=367)]
    pub synth_a_glide: RangedU8<127>,
    #[bits(368..=368)]
    pub synth_a_extern_enabled: bool,
    #[bits(399..=405)]
    pub synth_a_extern_program: RangedU8<127>,
    #[bits(414..=414)]
    pub synth_a_kb_hold: bool,
    #[bits(415..=415)]
    pub synth_a_arpeggiator_run_enabled: bool,
    #[bits(416..=417)]
    pub synth_a_arpeggiator_mode: RangedU8<3>,
    #[bits(418..=418)]
    pub synth_a_arp_pattern_enabled: bool,
    #[bits(419..=419)]
    pub synth_a_kb_sync_enabled: bool,
    #[bits(420..=426)]
    pub synth_a_arp_range_env: RangedU8<127>,
    #[bits(427..=434)]
    pub synth_a_arp_range_env_wheel: u8,
    #[bits(435..=442)]
    pub synth_a_arp_range_env_aftertouch: u8,
    #[bits(443..=450)]
    pub synth_a_arp_range_env_ctrl_pedal: u8,
    #[bits(451..=452)]
    pub synth_a_arp_direction: RangedU8<3>,
    #[bits(453..=453)]
    pub synth_a_arp_zigzag_enabled: bool,
    #[bits(454..=454)]
    pub synth_a_arp_master_clock_enabled: bool,
    #[bits(455..=461)]
    pub synth_a_arp_rate_time: RangedU8<127>,
    #[bits(462..=469)]
    pub synth_a_arp_rate_time_wheel: u8,
    #[bits(470..=477)]
    pub synth_a_arp_rate_time_aftertouch: u8,
    #[bits(478..=485)]
    pub synth_a_arp_rate_time_ctrl_pedal: u8,
    #[bits(486..=489)]
    pub synth_a_arp_pattern_length: RangedU8<15>,
    #[bits(490..=521)]
    pub synth_a_arpeggiator_accent: u32,
    #[bits(522..=553)]
    pub synth_a_arpeggiator_gate: u32,
    #[bits(554..=585)]
    pub synth_a_arpeggiator_pan: u32,
    #[bits(586..=587)]
    pub synth_a_unison_level: RangedU8<3>,
    #[bits(652..=656)]
    pub synth_a_vibrato_delay: RangedU8<31>,
    #[bits(696..=696)]
    pub synth_b_samples_analog: bool,
    #[bits(702..=713)]
    pub synth_b_sample_slot: RangedU16<4095>,
    #[bits(714..=745)]
    pub synth_b_sample_id: u32,
    #[bits(746..=749)]
    pub synth_b_kb_zones: RangedU8<15>,
    #[bits(750..=753)]
    pub synth_b_octave_shift: RangedU8<15>,
    #[bits(754..=754)]
    pub synth_b_pitch_stick_enabled: bool,
    #[bits(755..=758)]
    pub synth_b_pitch_stick_range: RangedU8<15>,
    #[bits(759..=759)]
    pub synth_b_sustain_pedal_enabled: bool,
    #[bits(760..=762)]
    pub synth_b_vibrato_mode: RangedU8<7>,
    #[bits(765..=765)]
    pub synth_b_legato_enabled: bool,
    #[bits(766..=766)]
    pub synth_b_mono_enabled: bool,
    #[bits(767..=768)]
    pub synth_b_voice_priority: RangedU8<3>,
    #[bits(769..=775)]
    pub synth_b_glide: RangedU8<127>,
    #[bits(776..=776)]
    pub synth_b_extern_enabled: bool,
    #[bits(807..=813)]
    pub synth_b_extern_program: RangedU8<127>,
    #[bits(822..=822)]
    pub synth_b_kb_hold: bool,
    #[bits(823..=823)]
    pub synth_b_arpeggiator_run_enabled: bool,
    #[bits(824..=825)]
    pub synth_b_arpeggiator_mode: RangedU8<3>,
    #[bits(826..=826)]
    pub synth_b_arp_pattern_enabled: bool,
    #[bits(827..=827)]
    pub synth_b_kb_sync_enabled: bool,
    #[bits(828..=834)]
    pub synth_b_arp_range_env: RangedU8<127>,
    #[bits(835..=842)]
    pub synth_b_arp_range_env_wheel: u8,
    #[bits(843..=850)]
    pub synth_b_arp_range_env_aftertouch: u8,
    #[bits(851..=858)]
    pub synth_b_arp_range_env_ctrl_pedal: u8,
    #[bits(859..=860)]
    pub synth_b_arp_direction: RangedU8<3>,
    #[bits(861..=861)]
    pub synth_b_arp_zigzag_enabled: bool,
    #[bits(862..=862)]
    pub synth_b_arp_master_clock_enabled: bool,
    #[bits(863..=869)]
    pub synth_b_arp_rate_time: RangedU8<127>,
    #[bits(870..=877)]
    pub synth_b_arp_rate_time_wheel: u8,
    #[bits(878..=885)]
    pub synth_b_arp_rate_time_aftertouch: u8,
    #[bits(886..=893)]
    pub synth_b_arp_rate_time_ctrl_pedal: u8,
    #[bits(894..=897)]
    pub synth_b_arp_pattern_length: RangedU8<15>,
    #[bits(898..=929)]
    pub synth_b_arpeggiator_accent: u32,
    #[bits(930..=961)]
    pub synth_b_arpeggiator_gate: u32,
    #[bits(962..=993)]
    pub synth_b_arpeggiator_pan: u32,
    #[bits(994..=995)]
    pub synth_b_unison_level: RangedU8<3>,
    #[bits(996..=1002)]
    pub synth_b_extern_cc_val1: RangedU8<127>,
    #[bits(1003..=1010)]
    pub synth_b_extern_cc_val1_wheel: u8,
    #[bits(1011..=1018)]
    pub synth_b_extern_cc_val1_aftertouch: u8,
    #[bits(1019..=1026)]
    pub synth_b_extern_cc_val1_ctrl_pedal: u8,
    #[bits(1027..=1033)]
    pub synth_b_extern_cc_val2: RangedU8<127>,
    #[bits(1034..=1041)]
    pub synth_b_extern_cc_val2_wheel: u8,
    #[bits(1042..=1049)]
    pub synth_b_extern_cc_val2_aftertouch: u8,
    #[bits(1050..=1057)]
    pub synth_b_extern_cc_val2_ctrl_pedal: u8,
    #[bits(1060..=1064)]
    pub synth_b_vibrato_delay: RangedU8<31>,
    #[bits(1104..=1104)]
    pub synth_c_samples_analog: bool,
    #[bits(1110..=1121)]
    pub synth_c_sample_slot: RangedU16<4095>,
    #[bits(1122..=1153)]
    pub synth_c_sample_id: u32,
    #[bits(1154..=1157)]
    pub synth_c_kb_zones: RangedU8<15>,
    #[bits(1158..=1161)]
    pub synth_c_octave_shift: RangedU8<15>,
    #[bits(1162..=1162)]
    pub synth_c_pitch_stick_enabled: bool,
    #[bits(1163..=1166)]
    pub synth_c_pitch_stick_range: RangedU8<15>,
    #[bits(1167..=1167)]
    pub synth_c_sustain_pedal_enabled: bool,
    #[bits(1168..=1170)]
    pub synth_c_vibrato_mode: RangedU8<7>,
    #[bits(1173..=1173)]
    pub synth_c_legato_enabled: bool,
    #[bits(1174..=1174)]
    pub synth_c_mono_enabled: bool,
    #[bits(1175..=1176)]
    pub synth_c_voice_priority: RangedU8<3>,
    #[bits(1177..=1183)]
    pub synth_c_glide: RangedU8<127>,
    #[bits(1184..=1184)]
    pub synth_c_extern_enabled: bool,
    #[bits(1215..=1221)]
    pub synth_c_extern_program: RangedU8<127>,
    #[bits(1230..=1230)]
    pub synth_c_kb_hold: bool,
    #[bits(1231..=1231)]
    pub synth_c_arpeggiator_run_enabled: bool,
    #[bits(1232..=1233)]
    pub synth_c_arpeggiator_mode: RangedU8<3>,
    #[bits(1234..=1234)]
    pub synth_c_arp_pattern_enabled: bool,
    #[bits(1235..=1235)]
    pub synth_c_kb_sync_enabled: bool,
    #[bits(1236..=1242)]
    pub synth_c_arp_range_env: RangedU8<127>,
    #[bits(1243..=1250)]
    pub synth_c_arp_range_env_wheel: u8,
    #[bits(1251..=1258)]
    pub synth_c_arp_range_env_aftertouch: u8,
    #[bits(1259..=1266)]
    pub synth_c_arp_range_env_ctrl_pedal: u8,
    #[bits(1267..=1268)]
    pub synth_c_arp_direction: RangedU8<3>,
    #[bits(1269..=1269)]
    pub synth_c_arp_zigzag_enabled: bool,
    #[bits(1270..=1270)]
    pub synth_c_arp_master_clock_enabled: bool,
    #[bits(1271..=1277)]
    pub synth_c_arp_rate_time: RangedU8<127>,
    #[bits(1278..=1285)]
    pub synth_c_arp_rate_time_wheel: u8,
    #[bits(1286..=1293)]
    pub synth_c_arp_rate_time_aftertouch: u8,
    #[bits(1294..=1301)]
    pub synth_c_arp_rate_time_ctrl_pedal: u8,
    #[bits(1302..=1305)]
    pub synth_c_arp_pattern_length: RangedU8<15>,
    #[bits(1306..=1337)]
    pub synth_c_arpeggiator_accent: u32,
    #[bits(1338..=1369)]
    pub synth_c_arpeggiator_gate: u32,
    #[bits(1370..=1401)]
    pub synth_c_arpeggiator_pan: u32,
    #[bits(1402..=1403)]
    pub synth_c_unison_level: RangedU8<3>,
    #[bits(1404..=1410)]
    pub synth_c_extern_cc_val1: RangedU8<127>,
    #[bits(1411..=1418)]
    pub synth_c_extern_cc_val1_wheel: u8,
    #[bits(1419..=1426)]
    pub synth_c_extern_cc_val1_aftertouch: u8,
    #[bits(1427..=1434)]
    pub synth_c_extern_cc_val1_ctrl_pedal: u8,
    #[bits(1435..=1441)]
    pub synth_c_extern_cc_val2: RangedU8<127>,
    #[bits(1442..=1449)]
    pub synth_c_extern_cc_val2_wheel: u8,
    #[bits(1450..=1457)]
    pub synth_c_extern_cc_val2_aftertouch: u8,
    #[bits(1458..=1465)]
    pub synth_c_extern_cc_val2_ctrl_pedal: u8,
    #[bits(1468..=1472)]
    pub synth_c_vibrato_delay: RangedU8<31>,
    #[bits(1514..=1515)]
    pub synth_a_analog_type_knob_1: RangedU8<3>,
    #[bits(1519..=1521)]
    pub synth_a_analog_cat_knob_2: RangedU8<7>,
    #[bits(1523..=1528)]
    pub synth_a_analog_wave_partial_knob_3: RangedU8<63>,
    #[bits(1529..=1535)]
    pub synth_a_osc_ctrl: RangedU8<127>,
    #[bits(1536..=1543)]
    pub synth_a_osc_ctrl_wheel: u8,
    #[bits(1544..=1551)]
    pub synth_a_osc_ctrl_aftertouch: u8,
    #[bits(1552..=1559)]
    pub synth_a_osc_ctrl_ctrl_pedal: u8,
    #[bits(1560..=1566)]
    pub synth_a_pitch_fine: RangedU8<127>,
    #[bits(1567..=1572)]
    pub synth_a_pitch_coarse: RangedU8<63>,
    #[bits(1575..=1581)]
    pub synth_a_osc_env_attack: RangedU8<127>,
    #[bits(1582..=1588)]
    pub synth_a_osc_env_decay: RangedU8<127>,
    #[bits(1589..=1595)]
    pub synth_a_osc_env_release: RangedU8<127>,
    #[bits(1596..=1602)]
    pub synth_a_osc_env_amount: RangedU8<127>,
    #[bits(1603..=1610)]
    pub synth_a_osc_env_amount_wheel: u8,
    #[bits(1611..=1618)]
    pub synth_a_osc_env_amount_aftertouch: u8,
    #[bits(1619..=1626)]
    pub synth_a_osc_env_amount_ctrl_pedal: u8,
    #[bits(1627..=1627)]
    pub synth_a_osc_env_to_pitch_enabled: bool,
    #[bits(1628..=1628)]
    pub synth_a_osc_env_velocity_enabled: bool,
    #[bits(1629..=1630)]
    pub synth_a_sample_options: RangedU8<3>,
    #[bits(1631..=1632)]
    pub synth_a_lfo_target: RangedU8<3>,
    #[bits(1633..=1635)]
    pub synth_a_lfo_shape: RangedU8<7>,
    #[bits(1636..=1636)]
    pub synth_a_lfo_master_clock_enabled: bool,
    #[bits(1637..=1643)]
    pub synth_a_lfo_rate_time: RangedU8<127>,
    #[bits(1644..=1651)]
    pub synth_a_lfo_rate_time_wheel: u8,
    #[bits(1652..=1659)]
    pub synth_a_lfo_rate_time_aftertouch: u8,
    #[bits(1660..=1667)]
    pub synth_a_lfo_rate_time_ctrl_pedal: u8,
    #[bits(1668..=1674)]
    pub synth_a_lfo_mod_amount: RangedU8<127>,
    #[bits(1675..=1682)]
    pub synth_a_lfo_mod_amount_wheel: u8,
    #[bits(1683..=1690)]
    pub synth_a_lfo_mod_amount_aftertouch: u8,
    #[bits(1691..=1698)]
    pub synth_a_lfo_mod_amount_ctrl_pedal: u8,
    #[bits(1699..=1705)]
    pub synth_a_amp_env_attack: RangedU8<127>,
    #[bits(1706..=1712)]
    pub synth_a_amp_env_decay: RangedU8<127>,
    #[bits(1713..=1719)]
    pub synth_a_amp_env_release: RangedU8<127>,
    #[bits(1720..=1721)]
    pub synth_a_amp_env_velocity: RangedU8<3>,
    #[bits(1722..=1724)]
    pub synth_a_filter_type: RangedU8<7>,
    #[bits(1725..=1731)]
    pub synth_a_filter_freq: RangedU8<127>,
    #[bits(1732..=1739)]
    pub synth_a_filter_freq_wheel: u8,
    #[bits(1740..=1747)]
    pub synth_a_filter_freq_aftertouch: u8,
    #[bits(1748..=1755)]
    pub synth_a_filter_freq_ctrl_pedal: u8,
    #[bits(1756..=1762)]
    pub synth_a_filter_resonance_freq_hp: RangedU8<127>,
    #[bits(1763..=1770)]
    pub synth_a_filter_resonance_wheel: u8,
    #[bits(1771..=1778)]
    pub synth_a_filter_resonance_aftertouch: u8,
    #[bits(1779..=1786)]
    pub synth_a_filter_resonance_ctrl_pedal: u8,
    #[bits(1787..=1788)]
    pub synth_a_filter_track: RangedU8<3>,
    #[bits(1789..=1790)]
    pub synth_a_filter_drive: RangedU8<3>,
    #[bits(1791..=1797)]
    pub synth_a_filter_env_amount: RangedU8<127>,
    #[bits(1798..=1805)]
    pub synth_a_filter_env_amount_wheel: u8,
    #[bits(1806..=1813)]
    pub synth_a_filter_env_amount_aftertouch: u8,
    #[bits(1814..=1821)]
    pub synth_a_filter_env_amount_ctrl_pedal: u8,
    #[bits(1822..=1828)]
    pub synth_a_filter_env_attack: RangedU8<127>,
    #[bits(1829..=1835)]
    pub synth_a_filter_env_decay: RangedU8<127>,
    #[bits(1836..=1842)]
    pub synth_a_filter_env_release: RangedU8<127>,
    #[bits(1843..=1843)]
    pub synth_a_filter_velocity_enabled: bool,
    #[bits(1844..=1844)]
    pub synth_a_filter_enabled: bool,
    #[bits(1845..=1851)]
    pub synth_a_vibrato_rate: RangedU8<127>,
    #[bits(1852..=1857)]
    pub synth_a_vibrato_amount: RangedU8<63>,
    #[bits(1858..=1858)]
    pub synth_a_sample_bright_enabled: bool,
    #[bits(1898..=1899)]
    pub synth_b_analog_type_knob_1: RangedU8<3>,
    #[bits(1903..=1905)]
    pub synth_b_analog_cat_knob_2: RangedU8<7>,
    #[bits(1907..=1912)]
    pub synth_b_analog_wave_partial_knob_3: RangedU8<63>,
    #[bits(1913..=1919)]
    pub synth_b_osc_ctrl: RangedU8<127>,
    #[bits(1920..=1927)]
    pub synth_b_osc_ctrl_wheel: u8,
    #[bits(1928..=1935)]
    pub synth_b_osc_ctrl_aftertouch: u8,
    #[bits(1936..=1943)]
    pub synth_b_osc_ctrl_ctrl_pedal: u8,
    #[bits(1944..=1950)]
    pub synth_b_pitch_fine: RangedU8<127>,
    #[bits(1951..=1956)]
    pub synth_b_pitch_coarse: RangedU8<63>,
    #[bits(1959..=1965)]
    pub synth_b_osc_env_attack: RangedU8<127>,
    #[bits(1966..=1972)]
    pub synth_b_osc_env_decay: RangedU8<127>,
    #[bits(1973..=1979)]
    pub synth_b_osc_env_release: RangedU8<127>,
    #[bits(1980..=1986)]
    pub synth_b_osc_env_amount: RangedU8<127>,
    #[bits(1987..=1994)]
    pub synth_b_osc_env_amount_wheel: u8,
    #[bits(1995..=2002)]
    pub synth_b_osc_env_amount_aftertouch: u8,
    #[bits(2003..=2010)]
    pub synth_b_osc_env_amount_ctrl_pedal: u8,
    #[bits(2011..=2011)]
    pub synth_b_osc_env_to_pitch_enabled: bool,
    #[bits(2012..=2012)]
    pub synth_b_osc_env_velocity_enabled: bool,
    #[bits(2013..=2014)]
    pub synth_b_sample_options: RangedU8<3>,
    #[bits(2015..=2016)]
    pub synth_b_lfo_target: RangedU8<3>,
    #[bits(2017..=2019)]
    pub synth_b_lfo_shape: RangedU8<7>,
    #[bits(2020..=2020)]
    pub synth_b_lfo_master_clock_enabled: bool,
    #[bits(2021..=2027)]
    pub synth_b_lfo_rate_time: RangedU8<127>,
    #[bits(2028..=2035)]
    pub synth_b_lfo_rate_time_wheel: u8,
    #[bits(2036..=2043)]
    pub synth_b_lfo_rate_time_aftertouch: u8,
    #[bits(2044..=2051)]
    pub synth_b_lfo_rate_time_ctrl_pedal: u8,
    #[bits(2052..=2058)]
    pub synth_b_lfo_mod_amount: RangedU8<127>,
    #[bits(2059..=2066)]
    pub synth_b_lfo_mod_amount_wheel: u8,
    #[bits(2067..=2074)]
    pub synth_b_lfo_mod_amount_aftertouch: u8,
    #[bits(2075..=2082)]
    pub synth_b_lfo_mod_amount_ctrl_pedal: u8,
    #[bits(2083..=2089)]
    pub synth_b_amp_env_attack: RangedU8<127>,
    #[bits(2090..=2096)]
    pub synth_b_amp_env_decay: RangedU8<127>,
    #[bits(2097..=2103)]
    pub synth_b_amp_env_release: RangedU8<127>,
    #[bits(2104..=2105)]
    pub synth_b_amp_env_velocity: RangedU8<3>,
    #[bits(2106..=2108)]
    pub synth_b_filter_type: RangedU8<7>,
    #[bits(2109..=2115)]
    pub synth_b_filter_freq: RangedU8<127>,
    #[bits(2116..=2123)]
    pub synth_b_filter_freq_wheel: u8,
    #[bits(2124..=2131)]
    pub synth_b_filter_freq_aftertouch: u8,
    #[bits(2132..=2139)]
    pub synth_b_filter_freq_ctrl_pedal: u8,
    #[bits(2140..=2146)]
    pub synth_b_filter_resonance_freq_hp: RangedU8<127>,
    #[bits(2147..=2154)]
    pub synth_b_filter_resonance_wheel: u8,
    #[bits(2155..=2162)]
    pub synth_b_filter_resonance_aftertouch: u8,
    #[bits(2163..=2170)]
    pub synth_b_filter_resonance_ctrl_pedal: u8,
    #[bits(2171..=2172)]
    pub synth_b_filter_track: RangedU8<3>,
    #[bits(2173..=2174)]
    pub synth_b_filter_drive: RangedU8<3>,
    #[bits(2175..=2181)]
    pub synth_b_filter_env_amount: RangedU8<127>,
    #[bits(2182..=2189)]
    pub synth_b_filter_env_amount_wheel: u8,
    #[bits(2190..=2197)]
    pub synth_b_filter_env_amount_aftertouch: u8,
    #[bits(2198..=2205)]
    pub synth_b_filter_env_amount_ctrl_pedal: u8,
    #[bits(2206..=2212)]
    pub synth_b_filter_env_attack: RangedU8<127>,
    #[bits(2213..=2219)]
    pub synth_b_filter_env_decay: RangedU8<127>,
    #[bits(2220..=2226)]
    pub synth_b_filter_env_release: RangedU8<127>,
    #[bits(2227..=2227)]
    pub synth_b_filter_velocity_enabled: bool,
    #[bits(2228..=2228)]
    pub synth_b_filter_enabled: bool,
    #[bits(2229..=2235)]
    pub synth_b_vibrato_rate: RangedU8<127>,
    #[bits(2236..=2241)]
    pub synth_b_vibrato_amount: RangedU8<63>,
    #[bits(2242..=2242)]
    pub synth_b_sample_bright_enabled: bool,
    #[bits(2282..=2283)]
    pub synth_c_analog_type_knob_1: RangedU8<3>,
    #[bits(2287..=2289)]
    pub synth_c_analog_cat_knob_2: RangedU8<7>,
    #[bits(2291..=2296)]
    pub synth_c_analog_wave_partial_knob_3: RangedU8<63>,
    #[bits(2297..=2303)]
    pub synth_c_osc_ctrl: RangedU8<127>,
    #[bits(2304..=2311)]
    pub synth_c_osc_ctrl_wheel: u8,
    #[bits(2312..=2319)]
    pub synth_c_osc_ctrl_aftertouch: u8,
    #[bits(2320..=2327)]
    pub synth_c_osc_ctrl_ctrl_pedal: u8,
    #[bits(2328..=2334)]
    pub synth_c_pitch_fine: RangedU8<127>,
    #[bits(2335..=2340)]
    pub synth_c_pitch_coarse: RangedU8<63>,
    #[bits(2343..=2349)]
    pub synth_c_osc_env_attack: RangedU8<127>,
    #[bits(2350..=2356)]
    pub synth_c_osc_env_decay: RangedU8<127>,
    #[bits(2357..=2363)]
    pub synth_c_osc_env_release: RangedU8<127>,
    #[bits(2364..=2370)]
    pub synth_c_osc_env_amount: RangedU8<127>,
    #[bits(2371..=2378)]
    pub synth_c_osc_env_amount_wheel: u8,
    #[bits(2379..=2386)]
    pub synth_c_osc_env_amount_aftertouch: u8,
    #[bits(2387..=2394)]
    pub synth_c_osc_env_amount_ctrl_pedal: u8,
    #[bits(2395..=2395)]
    pub synth_c_osc_env_to_pitch_enabled: bool,
    #[bits(2396..=2396)]
    pub synth_c_osc_env_velocity_enabled: bool,
    #[bits(2397..=2398)]
    pub synth_c_sample_options: RangedU8<3>,
    #[bits(2399..=2400)]
    pub synth_c_lfo_target: RangedU8<3>,
    #[bits(2401..=2403)]
    pub synth_c_lfo_shape: RangedU8<7>,
    #[bits(2404..=2404)]
    pub synth_c_lfo_master_clock_enabled: bool,
    #[bits(2405..=2411)]
    pub synth_c_lfo_rate_time: RangedU8<127>,
    #[bits(2412..=2419)]
    pub synth_c_lfo_rate_time_wheel: u8,
    #[bits(2420..=2427)]
    pub synth_c_lfo_rate_time_aftertouch: u8,
    #[bits(2428..=2435)]
    pub synth_c_lfo_rate_time_ctrl_pedal: u8,
    #[bits(2436..=2442)]
    pub synth_c_lfo_mod_amount: RangedU8<127>,
    #[bits(2443..=2450)]
    pub synth_c_lfo_mod_amount_wheel: u8,
    #[bits(2451..=2458)]
    pub synth_c_lfo_mod_amount_aftertouch: u8,
    #[bits(2459..=2466)]
    pub synth_c_lfo_mod_amount_ctrl_pedal: u8,
    #[bits(2467..=2473)]
    pub synth_c_amp_env_attack: RangedU8<127>,
    #[bits(2474..=2480)]
    pub synth_c_amp_env_decay: RangedU8<127>,
    #[bits(2481..=2487)]
    pub synth_c_amp_env_release: RangedU8<127>,
    #[bits(2488..=2489)]
    pub synth_c_amp_env_velocity: RangedU8<3>,
    #[bits(2490..=2492)]
    pub synth_c_filter_type: RangedU8<7>,
    #[bits(2493..=2499)]
    pub synth_c_filter_freq: RangedU8<127>,
    #[bits(2500..=2507)]
    pub synth_c_filter_freq_wheel: u8,
    #[bits(2508..=2515)]
    pub synth_c_filter_freq_aftertouch: u8,
    #[bits(2516..=2523)]
    pub synth_c_filter_freq_ctrl_pedal: u8,
    #[bits(2524..=2530)]
    pub synth_c_filter_resonance_freq_hp: RangedU8<127>,
    #[bits(2531..=2538)]
    pub synth_c_filter_resonance_wheel: u8,
    #[bits(2539..=2546)]
    pub synth_c_filter_resonance_aftertouch: u8,
    #[bits(2547..=2554)]
    pub synth_c_filter_resonance_ctrl_pedal: u8,
    #[bits(2555..=2556)]
    pub synth_c_filter_track: RangedU8<3>,
    #[bits(2557..=2558)]
    pub synth_c_filter_drive: RangedU8<3>,
    #[bits(2559..=2565)]
    pub synth_c_filter_env_amount: RangedU8<127>,
    #[bits(2566..=2573)]
    pub synth_c_filter_env_amount_wheel: u8,
    #[bits(2574..=2581)]
    pub synth_c_filter_env_amount_aftertouch: u8,
    #[bits(2582..=2589)]
    pub synth_c_filter_env_amount_ctrl_pedal: u8,
    #[bits(2590..=2596)]
    pub synth_c_filter_env_attack: RangedU8<127>,
    #[bits(2597..=2603)]
    pub synth_c_filter_env_decay: RangedU8<127>,
    #[bits(2604..=2610)]
    pub synth_c_filter_env_release: RangedU8<127>,
    #[bits(2611..=2611)]
    pub synth_c_filter_velocity_enabled: bool,
    #[bits(2612..=2612)]
    pub synth_c_filter_enabled: bool,
    #[bits(2613..=2619)]
    pub synth_c_vibrato_rate: RangedU8<127>,
    #[bits(2620..=2625)]
    pub synth_c_vibrato_amount: RangedU8<63>,
    #[bits(2626..=2626)]
    pub synth_c_sample_bright_enabled: bool,
    #[bits(2664..=2664)]
    pub synth_a_fx_mod_1_enabled: bool,
    #[bits(2665..=2665)]
    pub synth_a_fx_mod_1_master_clock_enabled: bool,
    #[bits(2666..=2672)]
    pub synth_a_fx_mod_1_rate: RangedU8<127>,
    #[bits(2673..=2680)]
    pub synth_a_fx_mod_1_rate_wheel: u8,
    #[bits(2681..=2688)]
    pub synth_a_fx_mod_1_rate_aftertouch: u8,
    #[bits(2689..=2696)]
    pub synth_a_fx_mod_1_rate_ctrl_pedal: u8,
    #[bits(2697..=2703)]
    pub synth_a_fx_mod_1_amount: RangedU8<127>,
    #[bits(2704..=2711)]
    pub synth_a_fx_mod_1_amount_wheel: u8,
    #[bits(2712..=2719)]
    pub synth_a_fx_mod_1_amount_aftertouch: u8,
    #[bits(2720..=2727)]
    pub synth_a_fx_mod_1_amount_ctrl_pedal: u8,
    #[bits(2728..=2731)]
    pub synth_a_fx_mod_1_mode: RangedU8<15>,
    #[bits(2732..=2732)]
    pub synth_a_fx_mod_2_enabled: bool,
    #[bits(2733..=2739)]
    pub synth_a_fx_mod_2_rate: RangedU8<127>,
    #[bits(2740..=2747)]
    pub synth_a_fx_mod_2_rate_wheel: u8,
    #[bits(2748..=2755)]
    pub synth_a_fx_mod_2_rate_aftertouch: u8,
    #[bits(2756..=2763)]
    pub synth_a_fx_mod_2_rate_ctrl_pedal: u8,
    #[bits(2764..=2770)]
    pub synth_a_fx_mod_2_amount: RangedU8<127>,
    #[bits(2771..=2778)]
    pub synth_a_fx_mod_2_amount_wheel: u8,
    #[bits(2779..=2786)]
    pub synth_a_fx_mod_2_amount_aftertouch: u8,
    #[bits(2787..=2794)]
    pub synth_a_fx_mod_2_amount_ctrl_pedal: u8,
    #[bits(2795..=2798)]
    pub synth_a_fx_mod_2_mode: RangedU8<15>,
    #[bits(2799..=2799)]
    pub synth_a_fx_amp_sim_eq_enabled: bool,
    #[bits(2800..=2806)]
    pub synth_a_fx_amp_sim_eq_treb: RangedU8<127>,
    #[bits(2807..=2813)]
    pub synth_a_fx_amp_sim_eq_mid: RangedU8<127>,
    #[bits(2814..=2820)]
    pub synth_a_fx_amp_sim_eq_bass: RangedU8<127>,
    #[bits(2821..=2827)]
    pub synth_a_fx_amp_sim_eq_freq: RangedU8<127>,
    #[bits(2828..=2835)]
    pub synth_a_fx_amp_sim_eq_freq_wheel: u8,
    #[bits(2836..=2843)]
    pub synth_a_fx_amp_sim_eq_freq_aftertouch: u8,
    #[bits(2844..=2851)]
    pub synth_a_fx_amp_sim_eq_freq_ctrl_pedal: u8,
    #[bits(2852..=2858)]
    pub synth_a_fx_amp_sim_eq_drive: RangedU8<127>,
    #[bits(2859..=2866)]
    pub synth_a_fx_amp_sim_eq_drive_wheel: u8,
    #[bits(2867..=2874)]
    pub synth_a_fx_amp_sim_eq_drive_aftertouch: u8,
    #[bits(2875..=2882)]
    pub synth_a_fx_amp_sim_eq_drive_ctrl_pedal: u8,
    #[bits(2887..=2887)]
    pub synth_a_fx_comp_enabled: bool,
    #[bits(2888..=2894)]
    pub synth_a_fx_comp_amount: RangedU8<127>,
    #[bits(2895..=2895)]
    pub synth_a_fx_comp_response: bool,
    #[bits(2896..=2896)]
    pub synth_a_fx_delay_enabled: bool,
    #[bits(2897..=2897)]
    pub synth_a_fx_delay_tempo_master_clock_enabled: bool,
    #[bits(2898..=2904)]
    pub synth_a_fx_delay_tempo: RangedU8<127>,
    #[bits(2912..=2919)]
    pub synth_a_fx_delay_tempo_wheel: u8,
    #[bits(2920..=2927)]
    pub synth_a_fx_delay_tempo_aftertouch: u8,
    #[bits(2928..=2935)]
    pub synth_a_fx_delay_tempo_ctrl_pedal: u8,
    #[bits(2957..=2963)]
    pub synth_a_fx_delay_mix: RangedU8<127>,
    #[bits(2964..=2971)]
    pub synth_a_fx_delay_mix_wheel: u8,
    #[bits(2972..=2979)]
    pub synth_a_fx_delay_mix_aftertouch: u8,
    #[bits(2980..=2987)]
    pub synth_a_fx_delay_mix_ctrl_pedal: u8,
    #[bits(2988..=2988)]
    pub synth_a_fx_delay_normal_analog: bool,
    #[bits(2989..=2989)]
    pub synth_a_fx_delay_ping_pong_enabled: bool,
    #[bits(2990..=2991)]
    pub synth_a_fx_delay_filter_type: RangedU8<3>,
    #[bits(2992..=2998)]
    pub synth_a_fx_delay_feedback: RangedU8<127>,
    #[bits(2999..=3006)]
    pub synth_a_fx_delay_feedback_wheel: u8,
    #[bits(3007..=3014)]
    pub synth_a_fx_delay_feedback_aftertouch: u8,
    #[bits(3015..=3022)]
    pub synth_a_fx_delay_feedback_ctrl_pedal: u8,
    #[bits(3023..=3026)]
    pub synth_a_fx_delay_effects: RangedU8<15>,
    #[bits(3027..=3027)]
    pub synth_a_fx_reverb_enabled: bool,
    #[bits(3028..=3034)]
    pub synth_a_fx_reverb_amount: RangedU8<127>,
    #[bits(3035..=3042)]
    pub synth_a_fx_reverb_amount_wheel: u8,
    #[bits(3043..=3050)]
    pub synth_a_fx_reverb_amount_aftertouch: u8,
    #[bits(3051..=3058)]
    pub synth_a_fx_reverb_amount_ctrl_pedal: u8,
    #[bits(3059..=3060)]
    pub synth_a_fx_reverb_dark_bright: RangedU8<3>,
    #[bits(3061..=3064)]
    pub synth_a_fx_reverb_type: RangedU8<15>,
    #[bits(3069..=3072)]
    pub synth_a_fx_amp_sim_eq_mode: RangedU8<15>,
    #[bits(3104..=3104)]
    pub synth_b_fx_mod_1_enabled: bool,
    #[bits(3105..=3105)]
    pub synth_b_fx_mod_1_master_clock_enabled: bool,
    #[bits(3106..=3112)]
    pub synth_b_fx_mod_1_rate: RangedU8<127>,
    #[bits(3113..=3120)]
    pub synth_b_fx_mod_1_rate_wheel: u8,
    #[bits(3121..=3128)]
    pub synth_b_fx_mod_1_rate_aftertouch: u8,
    #[bits(3129..=3136)]
    pub synth_b_fx_mod_1_rate_ctrl_pedal: u8,
    #[bits(3137..=3143)]
    pub synth_b_fx_mod_1_amount: RangedU8<127>,
    #[bits(3144..=3151)]
    pub synth_b_fx_mod_1_amount_wheel: u8,
    #[bits(3152..=3159)]
    pub synth_b_fx_mod_1_amount_aftertouch: u8,
    #[bits(3160..=3167)]
    pub synth_b_fx_mod_1_amount_ctrl_pedal: u8,
    #[bits(3168..=3171)]
    pub synth_b_fx_mod_1_mode: RangedU8<15>,
    #[bits(3172..=3172)]
    pub synth_b_fx_mod_2_enabled: bool,
    #[bits(3173..=3179)]
    pub synth_b_fx_mod_2_rate: RangedU8<127>,
    #[bits(3180..=3187)]
    pub synth_b_fx_mod_2_rate_wheel: u8,
    #[bits(3188..=3195)]
    pub synth_b_fx_mod_2_rate_aftertouch: u8,
    #[bits(3196..=3203)]
    pub synth_b_fx_mod_2_rate_ctrl_pedal: u8,
    #[bits(3204..=3210)]
    pub synth_b_fx_mod_2_amount: RangedU8<127>,
    #[bits(3211..=3218)]
    pub synth_b_fx_mod_2_amount_wheel: u8,
    #[bits(3219..=3226)]
    pub synth_b_fx_mod_2_amount_aftertouch: u8,
    #[bits(3227..=3234)]
    pub synth_b_fx_mod_2_amount_ctrl_pedal: u8,
    #[bits(3235..=3238)]
    pub synth_b_fx_mod_2_mode: RangedU8<15>,
    #[bits(3239..=3239)]
    pub synth_b_fx_amp_sim_eq_enabled: bool,
    #[bits(3240..=3246)]
    pub synth_b_fx_amp_sim_eq_treb: RangedU8<127>,
    #[bits(3247..=3253)]
    pub synth_b_fx_amp_sim_eq_mid: RangedU8<127>,
    #[bits(3254..=3260)]
    pub synth_b_fx_amp_sim_eq_bass: RangedU8<127>,
    #[bits(3261..=3267)]
    pub synth_b_fx_amp_sim_eq_freq: RangedU8<127>,
    #[bits(3268..=3275)]
    pub synth_b_fx_amp_sim_eq_freq_wheel: u8,
    #[bits(3276..=3283)]
    pub synth_b_fx_amp_sim_eq_freq_aftertouch: u8,
    #[bits(3284..=3291)]
    pub synth_b_fx_amp_sim_eq_freq_ctrl_pedal: u8,
    #[bits(3292..=3298)]
    pub synth_b_fx_amp_sim_eq_drive: RangedU8<127>,
    #[bits(3299..=3306)]
    pub synth_b_fx_amp_sim_eq_drive_wheel: u8,
    #[bits(3307..=3314)]
    pub synth_b_fx_amp_sim_eq_drive_aftertouch: u8,
    #[bits(3315..=3322)]
    pub synth_b_fx_amp_sim_eq_drive_ctrl_pedal: u8,
    #[bits(3327..=3327)]
    pub synth_b_fx_comp_enabled: bool,
    #[bits(3328..=3334)]
    pub synth_b_fx_comp_amount: RangedU8<127>,
    #[bits(3335..=3335)]
    pub synth_b_fx_comp_response: bool,
    #[bits(3336..=3336)]
    pub synth_b_fx_delay_enabled: bool,
    #[bits(3337..=3337)]
    pub synth_b_fx_delay_tempo_master_clock_enabled: bool,
    #[bits(3338..=3344)]
    pub synth_b_fx_delay_tempo: RangedU8<127>,
    #[bits(3352..=3359)]
    pub synth_b_fx_delay_tempo_wheel: u8,
    #[bits(3360..=3367)]
    pub synth_b_fx_delay_tempo_aftertouch: u8,
    #[bits(3368..=3375)]
    pub synth_b_fx_delay_tempo_ctrl_pedal: u8,
    #[bits(3397..=3403)]
    pub synth_b_fx_delay_mix: RangedU8<127>,
    #[bits(3404..=3411)]
    pub synth_b_fx_delay_mix_wheel: u8,
    #[bits(3412..=3419)]
    pub synth_b_fx_delay_mix_aftertouch: u8,
    #[bits(3420..=3427)]
    pub synth_b_fx_delay_mix_ctrl_pedal: u8,
    #[bits(3428..=3428)]
    pub synth_b_fx_delay_normal_analog: bool,
    #[bits(3429..=3429)]
    pub synth_b_fx_delay_ping_pong_enabled: bool,
    #[bits(3430..=3431)]
    pub synth_b_fx_delay_filter_type: RangedU8<3>,
    #[bits(3432..=3438)]
    pub synth_b_fx_delay_feedback: RangedU8<127>,
    #[bits(3439..=3446)]
    pub synth_b_fx_delay_feedback_wheel: u8,
    #[bits(3447..=3454)]
    pub synth_b_fx_delay_feedback_aftertouch: u8,
    #[bits(3455..=3462)]
    pub synth_b_fx_delay_feedback_ctrl_pedal: u8,
    #[bits(3463..=3466)]
    pub synth_b_fx_delay_effects: RangedU8<15>,
    #[bits(3467..=3467)]
    pub synth_b_fx_reverb_enabled: bool,
    #[bits(3468..=3474)]
    pub synth_b_fx_reverb_amount: RangedU8<127>,
    #[bits(3475..=3482)]
    pub synth_b_fx_reverb_amount_wheel: u8,
    #[bits(3483..=3490)]
    pub synth_b_fx_reverb_amount_aftertouch: u8,
    #[bits(3491..=3498)]
    pub synth_b_fx_reverb_amount_ctrl_pedal: u8,
    #[bits(3499..=3500)]
    pub synth_b_fx_reverb_dark_bright: RangedU8<3>,
    #[bits(3501..=3504)]
    pub synth_b_fx_reverb_type: RangedU8<15>,
    #[bits(3509..=3512)]
    pub synth_b_fx_amp_sim_eq_mode: RangedU8<15>,
    #[bits(3544..=3544)]
    pub synth_c_fx_mod_1_enabled: bool,
    #[bits(3545..=3545)]
    pub synth_c_fx_mod_1_master_clock_enabled: bool,
    #[bits(3546..=3552)]
    pub synth_c_fx_mod_1_rate: RangedU8<127>,
    #[bits(3553..=3560)]
    pub synth_c_fx_mod_1_rate_wheel: u8,
    #[bits(3561..=3568)]
    pub synth_c_fx_mod_1_rate_aftertouch: u8,
    #[bits(3569..=3576)]
    pub synth_c_fx_mod_1_rate_ctrl_pedal: u8,
    #[bits(3577..=3583)]
    pub synth_c_fx_mod_1_amount: RangedU8<127>,
    #[bits(3584..=3591)]
    pub synth_c_fx_mod_1_amount_wheel: u8,
    #[bits(3592..=3599)]
    pub synth_c_fx_mod_1_amount_aftertouch: u8,
    #[bits(3600..=3607)]
    pub synth_c_fx_mod_1_amount_ctrl_pedal: u8,
    #[bits(3608..=3611)]
    pub synth_c_fx_mod_1_mode: RangedU8<15>,
    #[bits(3612..=3612)]
    pub synth_c_fx_mod_2_enabled: bool,
    #[bits(3613..=3619)]
    pub synth_c_fx_mod_2_rate: RangedU8<127>,
    #[bits(3620..=3627)]
    pub synth_c_fx_mod_2_rate_wheel: u8,
    #[bits(3628..=3635)]
    pub synth_c_fx_mod_2_rate_aftertouch: u8,
    #[bits(3636..=3643)]
    pub synth_c_fx_mod_2_rate_ctrl_pedal: u8,
    #[bits(3644..=3650)]
    pub synth_c_fx_mod_2_amount: RangedU8<127>,
    #[bits(3651..=3658)]
    pub synth_c_fx_mod_2_amount_wheel: u8,
    #[bits(3659..=3666)]
    pub synth_c_fx_mod_2_amount_aftertouch: u8,
    #[bits(3667..=3674)]
    pub synth_c_fx_mod_2_amount_ctrl_pedal: u8,
    #[bits(3675..=3678)]
    pub synth_c_fx_mod_2_mode: RangedU8<15>,
    #[bits(3679..=3679)]
    pub synth_c_fx_amp_sim_eq_enabled: bool,
    #[bits(3680..=3686)]
    pub synth_c_fx_amp_sim_eq_treb: RangedU8<127>,
    #[bits(3687..=3693)]
    pub synth_c_fx_amp_sim_eq_mid: RangedU8<127>,
    #[bits(3694..=3700)]
    pub synth_c_fx_amp_sim_eq_bass: RangedU8<127>,
    #[bits(3701..=3707)]
    pub synth_c_fx_amp_sim_eq_freq: RangedU8<127>,
    #[bits(3708..=3715)]
    pub synth_c_fx_amp_sim_eq_freq_wheel: u8,
    #[bits(3716..=3723)]
    pub synth_c_fx_amp_sim_eq_freq_aftertouch: u8,
    #[bits(3724..=3731)]
    pub synth_c_fx_amp_sim_eq_freq_ctrl_pedal: u8,
    #[bits(3732..=3738)]
    pub synth_c_fx_amp_sim_eq_drive: RangedU8<127>,
    #[bits(3739..=3746)]
    pub synth_c_fx_amp_sim_eq_drive_wheel: u8,
    #[bits(3747..=3754)]
    pub synth_c_fx_amp_sim_eq_drive_aftertouch: u8,
    #[bits(3755..=3762)]
    pub synth_c_fx_amp_sim_eq_drive_ctrl_pedal: u8,
    #[bits(3767..=3767)]
    pub synth_c_fx_comp_enabled: bool,
    #[bits(3768..=3774)]
    pub synth_c_fx_comp_amount: RangedU8<127>,
    #[bits(3775..=3775)]
    pub synth_c_fx_comp_response: bool,
    #[bits(3776..=3776)]
    pub synth_c_fx_delay_enabled: bool,
    #[bits(3777..=3777)]
    pub synth_c_fx_delay_tempo_master_clock_enabled: bool,
    #[bits(3778..=3784)]
    pub synth_c_fx_delay_tempo: RangedU8<127>,
    #[bits(3792..=3799)]
    pub synth_c_fx_delay_tempo_wheel: u8,
    #[bits(3800..=3807)]
    pub synth_c_fx_delay_tempo_aftertouch: u8,
    #[bits(3808..=3815)]
    pub synth_c_fx_delay_tempo_ctrl_pedal: u8,
    #[bits(3837..=3843)]
    pub synth_c_fx_delay_mix: RangedU8<127>,
    #[bits(3844..=3851)]
    pub synth_c_fx_delay_mix_wheel: u8,
    #[bits(3852..=3859)]
    pub synth_c_fx_delay_mix_aftertouch: u8,
    #[bits(3860..=3867)]
    pub synth_c_fx_delay_mix_ctrl_pedal: u8,
    #[bits(3868..=3868)]
    pub synth_c_fx_delay_normal_analog: bool,
    #[bits(3869..=3869)]
    pub synth_c_fx_delay_ping_pong_enabled: bool,
    #[bits(3870..=3871)]
    pub synth_c_fx_delay_filter_type: RangedU8<3>,
    #[bits(3872..=3878)]
    pub synth_c_fx_delay_feedback: RangedU8<127>,
    #[bits(3879..=3886)]
    pub synth_c_fx_delay_feedback_wheel: u8,
    #[bits(3887..=3894)]
    pub synth_c_fx_delay_feedback_aftertouch: u8,
    #[bits(3895..=3902)]
    pub synth_c_fx_delay_feedback_ctrl_pedal: u8,
    #[bits(3903..=3906)]
    pub synth_c_fx_delay_effects: RangedU8<15>,
    #[bits(3907..=3907)]
    pub synth_c_fx_reverb_enabled: bool,
    #[bits(3908..=3914)]
    pub synth_c_fx_reverb_amount: RangedU8<127>,
    #[bits(3915..=3922)]
    pub synth_c_fx_reverb_amount_wheel: u8,
    #[bits(3923..=3930)]
    pub synth_c_fx_reverb_amount_aftertouch: u8,
    #[bits(3931..=3938)]
    pub synth_c_fx_reverb_amount_ctrl_pedal: u8,
    #[bits(3939..=3940)]
    pub synth_c_fx_reverb_dark_bright: RangedU8<3>,
    #[bits(3941..=3944)]
    pub synth_c_fx_reverb_type: RangedU8<15>,
    #[bits(3949..=3952)]
    pub synth_c_fx_amp_sim_eq_mode: RangedU8<15>,
}

pub fn read_from(reader: &mut (impl Read + Seek)) -> Result<Cbin<SynthPreset>, Error> {
    let file: Cbin<SynthPreset> = cbin::read(reader, FORMAT)?;
    crate::formats::known_version(FORMAT, file.header.version, KNOWN_VERSIONS)?;
    Ok(file)
}
