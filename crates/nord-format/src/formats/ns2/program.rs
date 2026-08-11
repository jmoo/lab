//! The Stage 2 program body (`.ns2p`, `.ns2l`): 521 bytes, every documented
//! parameter placed.
//!
//! The program-wide globals were decoded first and by hand; everything else — the
//! organ's B3, Vox and Farfisa drawbar banks, the piano, synth, extern and the
//! effects chain — comes from the byte maps.
//!
//! ⚠️ Stage 2 files are **type-0** containers, where the Stage 3's are type-1. The
//! byte maps number both in the type-1 layout; since type-0 differs only by
//! omitting `0x18..0x2b`, the body is the same either way and a documented offset
//! is `doc - 0x2c` in both.
//!
//! Values are raw except where the documentation enumerates them; see [the module
//! docs](super) for what that ceiling is and why.

use crate::cbin::{self, Cbin, Header};
use crate::components::{
    sparse_enum, Effect1Type, Effect2Type, MasterTempo, ProgramCategory, ReverbType, SplitNote,
    StageTranspose,
};
use crate::error::Error;
use crate::types::{RangedU16, RangedU8};
use std::io::{Read, Seek};

pub const FORMAT: &str = "ns2p";
/// Schema versions this build's field offsets have been validated against. The
/// corpus factory banks hold 6 and 7; the ns3-program-viewer accepts 2 through 7
/// with the same offsets.
pub const KNOWN_VERSIONS: &[u32] = &[2, 3, 4, 5, 6, 7];
pub const BODY_LEN: usize = 521;

/// The program-wide globals at the head of the body. Bits are MSB-first from body
/// byte 0 (`0x2c` in a type-1 file), so byte 0x02 bit 5 is bit 18.
#[nord_bits_derive::bitbody(521)]
pub struct Program {
    #[bits(18..=18)]
    pub dual_keyboard: bool,
    /// The Low split point in a three-zone split, or the only one in two zones.
    #[bits(20..=23)]
    pub split_low_note: SplitNote,
    /// ⚠️ Never `F2` (stored 0) on the panel: the High point's table starts at C3.
    #[bits(24..=27)]
    pub split_high_note: SplitNote,
    #[bits(28..=28)]
    pub split_three_zones: bool,
    #[bits(29..=29)]
    pub split_two_zones: bool,
    /// Touched at least once, not active: the untouched default stores 6 (= 0),
    /// and the EX factory live buffers hold the out-of-table 15.
    #[bits(34..=34)]
    pub transpose_enabled: bool,
    #[bits(35..=38)]
    pub transpose: StageTranspose,
    #[bits(43..=50)]
    pub master_clock: MasterTempo,
    // ── Generated from the Stage byte maps. Everything below is one
    //    field per documented run, in offset order.
    #[bits(16..=17)]
    pub slot_selection: RangedU8<3>,
    #[bits(33..=33)]
    pub organ_pitch_stick: bool,
    #[bits(64..=65)]
    pub organ_model: RangedU8<3>,
    #[bits(72..=74)]
    pub organ_b3_vibrato_mode: RangedU8<7>,
    #[bits(75..=75)]
    pub organ_b3_harmonic_third: bool,
    #[bits(76..=76)]
    pub organ_b3_decay_fast: bool,
    #[bits(77..=77)]
    pub organ_b3_volume_soft: bool,
    #[bits(89..=90)]
    pub organ_vox_vibrato_mode: RangedU8<3>,
    #[bits(91..=91)]
    pub organ_vox_vibrato_on: bool,
    #[bits(105..=106)]
    pub organ_farfisa_vibrato_mode: RangedU8<3>,
    #[bits(107..=107)]
    pub organ_farfisa_vibrato_on: bool,
    #[bits(120..=122)]
    pub piano_slot_detune: RangedU8<7>,
    #[bits(136..=136)]
    pub reverb_on: bool,
    #[bits(137..=139)]
    pub reverb_type: ReverbType,
    #[bits(140..=146)]
    pub reverb_amount: RangedU8<127>,
    #[bits(147..=147)]
    pub compressor_on: bool,
    #[bits(148..=154)]
    pub compressor_amount: RangedU8<127>,
    #[bits(155..=155)]
    pub rotary_speaker_on: bool,
    #[bits(156..=157)]
    pub rotary_speaker_source: RangedU8<3>,
    #[bits(158..=164)]
    pub rotary_speaker_drive: RangedU8<127>,
    #[bits(165..=165)]
    pub rotary_speaker_stop_mode: bool,
    #[bits(166..=166)]
    pub rotary_speaker_speed: bool,
    #[bits(167..=167)]
    pub rotary_speaker_speed_wheel: bool,
    #[bits(168..=168)]
    pub rotary_speaker_speed_aftertouch: bool,
    #[bits(169..=169)]
    pub rotary_speaker_speed_ctrl_pedal: bool,
    #[bits(184..=184)]
    pub organ_on: bool,
    #[bits(185..=192)]
    pub organ_volume_wheel: u8,
    #[bits(193..=200)]
    pub organ_volume_aftertouch: u8,
    #[bits(201..=208)]
    pub organ_volume_ctrl_pedal: u8,
    #[bits(209..=215)]
    pub organ_volume: RangedU8<127>,
    #[bits(216..=218)]
    pub organ_kb_zone: OrganKbZone,
    #[bits(219..=222)]
    pub organ_octave_shift: RangedU8<15>,
    #[bits(223..=223)]
    pub organ_sustain_pedal: bool,
    #[bits(224..=224)]
    pub piano_on: bool,
    #[bits(225..=232)]
    pub piano_volume_wheel: u8,
    #[bits(233..=240)]
    pub piano_volume_aftertouch: u8,
    #[bits(241..=248)]
    pub piano_volume_ctrl_pedal: u8,
    #[bits(249..=255)]
    pub piano_volume: RangedU8<127>,
    #[bits(256..=258)]
    pub piano_split_zones: PianoKbZone,
    #[bits(259..=262)]
    pub piano_octave_shift: RangedU8<15>,
    #[bits(263..=263)]
    pub piano_pitch_stick: bool,
    #[bits(264..=264)]
    pub piano_sustain_pedal: bool,
    #[bits(265..=265)]
    pub synth_on: bool,
    #[bits(266..=273)]
    pub synth_volume_wheel: u8,
    #[bits(274..=281)]
    pub synth_volume_aftertouch: u8,
    #[bits(282..=289)]
    pub synth_volume_ctrl_pedal: u8,
    #[bits(290..=296)]
    pub synth_volume: RangedU8<127>,
    #[bits(297..=299)]
    pub synth_kb_zone: SynthKbZone,
    #[bits(300..=303)]
    pub synth_octave_shift: RangedU8<15>,
    #[bits(304..=304)]
    pub synth_pitch_stick: bool,
    #[bits(305..=305)]
    pub synth_sustain_pedal: bool,
    #[bits(306..=306)]
    pub extern_on: bool,
    #[bits(338..=340)]
    pub extern_kb_zone: RangedU8<7>,
    #[bits(341..=344)]
    pub extern_octave_shift: RangedU8<15>,
    #[bits(345..=345)]
    pub extern_pitch_stick: bool,
    #[bits(346..=346)]
    pub extern_sustain_pedal: bool,
    #[bits(358..=359)]
    pub piano_program_output: RangedU8<3>,
    #[bits(361..=362)]
    pub synth_program_output: RangedU8<3>,
    #[bits(364..=365)]
    pub organ_program_output: RangedU8<3>,
    #[bits(366..=366)]
    pub organ_latch_pedal: bool,
    #[bits(367..=367)]
    pub organ_kb_gate: bool,
    #[bits(368..=368)]
    pub piano_latch_pedal: bool,
    #[bits(369..=369)]
    pub piano_kb_gate: bool,
    #[bits(370..=370)]
    pub synth_latch_pedal: bool,
    #[bits(371..=371)]
    pub synth_kb_gate: bool,
    #[bits(384..=384)]
    pub organ_b3_preset_2: bool,
    #[bits(392..=392)]
    pub organ_vox_vox_ii: bool,
    #[bits(400..=400)]
    pub organ_farfisa_preset_2: bool,
    #[bits(408..=412)]
    pub organ_b3_preset_1_drawbar_1_wheel: RangedU8<31>,
    #[bits(413..=417)]
    pub organ_b3_preset_1_drawbar_1_aftertouch: RangedU8<31>,
    #[bits(418..=422)]
    pub organ_b3_preset_1_drawbar_1_ctrl_pedal: RangedU8<31>,
    #[bits(423..=426)]
    pub organ_b3_preset_1_drawbar_1: RangedU8<15>,
    #[bits(427..=431)]
    pub organ_b3_preset_1_drawbar_2_wheel: RangedU8<31>,
    #[bits(432..=436)]
    pub organ_b3_preset_1_drawbar_2_aftertouch: RangedU8<31>,
    #[bits(437..=441)]
    pub organ_b3_preset_1_drawbar_2_ctrl_pedal: RangedU8<31>,
    #[bits(442..=445)]
    pub organ_b3_preset_1_drawbar_2: RangedU8<15>,
    #[bits(446..=450)]
    pub organ_b3_preset_1_drawbar_3_wheel: RangedU8<31>,
    #[bits(451..=455)]
    pub organ_b3_preset_1_drawbar_3_aftertouch: RangedU8<31>,
    #[bits(456..=460)]
    pub organ_b3_preset_1_drawbar_3_ctrl_pedal: RangedU8<31>,
    #[bits(461..=464)]
    pub organ_b3_preset_1_drawbar_3: RangedU8<15>,
    #[bits(465..=469)]
    pub organ_b3_preset_1_drawbar_4_wheel: RangedU8<31>,
    #[bits(470..=474)]
    pub organ_b3_preset_1_drawbar_4_aftertouch: RangedU8<31>,
    #[bits(475..=479)]
    pub organ_b3_preset_1_drawbar_4_ctrl_pedal: RangedU8<31>,
    #[bits(480..=483)]
    pub organ_b3_preset_1_drawbar_4: RangedU8<15>,
    #[bits(484..=488)]
    pub organ_b3_preset_1_drawbar_5_wheel: RangedU8<31>,
    #[bits(489..=493)]
    pub organ_b3_preset_1_drawbar_5_aftertouch: RangedU8<31>,
    #[bits(494..=498)]
    pub organ_b3_preset_1_drawbar_5_ctrl_pedal: RangedU8<31>,
    #[bits(499..=502)]
    pub organ_b3_preset_1_drawbar_5: RangedU8<15>,
    #[bits(503..=507)]
    pub organ_b3_preset_1_drawbar_6_wheel: RangedU8<31>,
    #[bits(508..=512)]
    pub organ_b3_preset_1_drawbar_6_aftertouch: RangedU8<31>,
    #[bits(513..=517)]
    pub organ_b3_preset_1_drawbar_6_ctrl_pedal: RangedU8<31>,
    #[bits(518..=521)]
    pub organ_b3_preset_1_drawbar_6: RangedU8<15>,
    #[bits(522..=526)]
    pub organ_b3_preset_1_drawbar_7_wheel: RangedU8<31>,
    #[bits(527..=531)]
    pub organ_b3_preset_1_drawbar_7_aftertouch: RangedU8<31>,
    #[bits(532..=536)]
    pub organ_b3_preset_1_drawbar_7_ctrl_pedal: RangedU8<31>,
    #[bits(537..=540)]
    pub organ_b3_preset_1_drawbar_7: RangedU8<15>,
    #[bits(541..=545)]
    pub organ_b3_preset_1_drawbar_8_wheel: RangedU8<31>,
    #[bits(546..=550)]
    pub organ_b3_preset_1_drawbar_8_aftertouch: RangedU8<31>,
    #[bits(551..=555)]
    pub organ_b3_preset_1_drawbar_8_ctrl_pedal: RangedU8<31>,
    #[bits(556..=559)]
    pub organ_b3_preset_1_drawbar_8: RangedU8<15>,
    #[bits(560..=564)]
    pub organ_b3_preset_1_drawbar_9_wheel: RangedU8<31>,
    #[bits(565..=569)]
    pub organ_b3_preset_1_drawbar_9_aftertouch: RangedU8<31>,
    #[bits(570..=574)]
    pub organ_b3_preset_1_drawbar_9_ctrl_pedal: RangedU8<31>,
    #[bits(575..=578)]
    pub organ_b3_preset_1_drawbar_9: RangedU8<15>,
    #[bits(579..=579)]
    pub organ_b3_preset_1_vibrato_chorus: bool,
    #[bits(580..=580)]
    pub organ_b3_preset_1_percussion: bool,
    #[bits(592..=596)]
    pub organ_vox_preset_1_drawbar_1_wheel: RangedU8<31>,
    #[bits(597..=601)]
    pub organ_vox_preset_1_drawbar_1_aftertouch: RangedU8<31>,
    #[bits(602..=606)]
    pub organ_vox_preset_1_drawbar_1_ctrl_pedal: RangedU8<31>,
    #[bits(607..=610)]
    pub organ_vox_preset_1_drawbar_1: RangedU8<15>,
    #[bits(611..=615)]
    pub organ_vox_preset_1_drawbar_2_wheel: RangedU8<31>,
    #[bits(616..=620)]
    pub organ_vox_preset_1_drawbar_2_aftertouch: RangedU8<31>,
    #[bits(621..=625)]
    pub organ_vox_preset_1_drawbar_2_ctrl_pedal: RangedU8<31>,
    #[bits(626..=629)]
    pub organ_vox_preset_1_drawbar_2: RangedU8<15>,
    #[bits(630..=634)]
    pub organ_vox_preset_1_drawbar_3_wheel: RangedU8<31>,
    #[bits(635..=639)]
    pub organ_vox_preset_1_drawbar_3_aftertouch: RangedU8<31>,
    #[bits(640..=644)]
    pub organ_vox_preset_1_drawbar_3_ctrl_pedal: RangedU8<31>,
    #[bits(645..=648)]
    pub organ_vox_preset_1_drawbar_3: RangedU8<15>,
    #[bits(649..=653)]
    pub organ_vox_preset_1_drawbar_4_wheel: RangedU8<31>,
    #[bits(654..=658)]
    pub organ_vox_preset_1_drawbar_4_aftertouch: RangedU8<31>,
    #[bits(659..=663)]
    pub organ_vox_preset_1_drawbar_4_ctrl_pedal: RangedU8<31>,
    #[bits(664..=667)]
    pub organ_vox_preset_1_drawbar_4: RangedU8<15>,
    #[bits(668..=672)]
    pub organ_vox_preset_1_drawbar_5_wheel: RangedU8<31>,
    #[bits(673..=677)]
    pub organ_vox_preset_1_drawbar_5_aftertouch: RangedU8<31>,
    #[bits(678..=682)]
    pub organ_vox_preset_1_drawbar_5_ctrl_pedal: RangedU8<31>,
    #[bits(683..=686)]
    pub organ_vox_preset_1_drawbar_5: RangedU8<15>,
    #[bits(687..=691)]
    pub organ_vox_preset_1_drawbar_6_wheel: RangedU8<31>,
    #[bits(692..=696)]
    pub organ_vox_preset_1_drawbar_6_aftertouch: RangedU8<31>,
    #[bits(697..=701)]
    pub organ_vox_preset_1_drawbar_6_ctrl_pedal: RangedU8<31>,
    #[bits(702..=705)]
    pub organ_vox_preset_1_drawbar_6: RangedU8<15>,
    #[bits(706..=710)]
    pub organ_vox_preset_1_drawbar_7_wheel: RangedU8<31>,
    #[bits(711..=715)]
    pub organ_vox_preset_1_drawbar_7_aftertouch: RangedU8<31>,
    #[bits(716..=720)]
    pub organ_vox_preset_1_drawbar_7_ctrl_pedal: RangedU8<31>,
    #[bits(721..=724)]
    pub organ_vox_preset_1_drawbar_7: RangedU8<15>,
    #[bits(725..=729)]
    pub organ_vox_preset_1_drawbar_8_wheel: RangedU8<31>,
    #[bits(730..=734)]
    pub organ_vox_preset_1_drawbar_8_aftertouch: RangedU8<31>,
    #[bits(735..=739)]
    pub organ_vox_preset_1_drawbar_8_ctrl_pedal: RangedU8<31>,
    #[bits(740..=743)]
    pub organ_vox_preset_1_drawbar_8: RangedU8<15>,
    #[bits(744..=748)]
    pub organ_vox_preset_1_drawbar_9_wheel: RangedU8<31>,
    #[bits(749..=753)]
    pub organ_vox_preset_1_drawbar_9_aftertouch: RangedU8<31>,
    #[bits(754..=758)]
    pub organ_vox_preset_1_drawbar_9_ctrl_pedal: RangedU8<31>,
    #[bits(759..=762)]
    pub organ_vox_preset_1_drawbar_9: RangedU8<15>,
    #[bits(776..=777)]
    pub organ_farfisa_preset_1_drawbar_1_wheel: RangedU8<3>,
    #[bits(778..=779)]
    pub organ_farfisa_preset_1_drawbar_1_aftertouch: RangedU8<3>,
    #[bits(780..=781)]
    pub organ_farfisa_preset_1_drawbar_1_ctrl_pedal: RangedU8<3>,
    #[bits(782..=782)]
    pub organ_farfisa_preset_1_drawbar_1: bool,
    #[bits(783..=784)]
    pub organ_farfisa_preset_1_drawbar_2_wheel: RangedU8<3>,
    #[bits(785..=786)]
    pub organ_farfisa_preset_1_drawbar_2_aftertouch: RangedU8<3>,
    #[bits(787..=788)]
    pub organ_farfisa_preset_1_drawbar_2_ctrl_pedal: RangedU8<3>,
    #[bits(789..=789)]
    pub organ_farfisa_preset_1_drawbar_2: bool,
    #[bits(790..=791)]
    pub organ_farfisa_preset_1_drawbar_3_wheel: RangedU8<3>,
    #[bits(792..=793)]
    pub organ_farfisa_preset_1_drawbar_3_aftertouch: RangedU8<3>,
    #[bits(794..=795)]
    pub organ_farfisa_preset_1_drawbar_3_ctrl_pedal: RangedU8<3>,
    #[bits(796..=796)]
    pub organ_farfisa_preset_1_drawbar_3: bool,
    #[bits(797..=798)]
    pub organ_farfisa_preset_1_drawbar_4_wheel: RangedU8<3>,
    #[bits(799..=800)]
    pub organ_farfisa_preset_1_drawbar_4_aftertouch: RangedU8<3>,
    #[bits(801..=802)]
    pub organ_farfisa_preset_1_drawbar_4_ctrl_pedal: RangedU8<3>,
    #[bits(803..=803)]
    pub organ_farfisa_preset_1_drawbar_4: bool,
    #[bits(804..=805)]
    pub organ_farfisa_preset_1_drawbar_5_wheel: RangedU8<3>,
    #[bits(806..=807)]
    pub organ_farfisa_preset_1_drawbar_5_aftertouch: RangedU8<3>,
    #[bits(808..=809)]
    pub organ_farfisa_preset_1_drawbar_5_ctrl_pedal: RangedU8<3>,
    #[bits(810..=810)]
    pub organ_farfisa_preset_1_drawbar_5: bool,
    #[bits(811..=812)]
    pub organ_farfisa_preset_1_drawbar_6_wheel: RangedU8<3>,
    #[bits(813..=814)]
    pub organ_farfisa_preset_1_drawbar_6_aftertouch: RangedU8<3>,
    #[bits(815..=816)]
    pub organ_farfisa_preset_1_drawbar_6_ctrl_pedal: RangedU8<3>,
    #[bits(817..=817)]
    pub organ_farfisa_preset_1_drawbar_6: bool,
    #[bits(818..=819)]
    pub organ_farfisa_preset_1_drawbar_7_wheel: RangedU8<3>,
    #[bits(820..=821)]
    pub organ_farfisa_preset_1_drawbar_7_aftertouch: RangedU8<3>,
    #[bits(822..=823)]
    pub organ_farfisa_preset_1_drawbar_7_ctrl_pedal: RangedU8<3>,
    #[bits(824..=824)]
    pub organ_farfisa_preset_1_drawbar_7: bool,
    #[bits(825..=826)]
    pub organ_farfisa_preset_1_drawbar_8_wheel: RangedU8<3>,
    #[bits(827..=828)]
    pub organ_farfisa_preset_1_drawbar_8_aftertouch: RangedU8<3>,
    #[bits(829..=830)]
    pub organ_farfisa_preset_1_drawbar_8_ctrl_pedal: RangedU8<3>,
    #[bits(831..=831)]
    pub organ_farfisa_preset_1_drawbar_8: bool,
    #[bits(832..=833)]
    pub organ_farfisa_preset_1_drawbar_9_wheel: RangedU8<3>,
    #[bits(834..=835)]
    pub organ_farfisa_preset_1_drawbar_9_aftertouch: RangedU8<3>,
    #[bits(836..=837)]
    pub organ_farfisa_preset_1_drawbar_9_ctrl_pedal: RangedU8<3>,
    #[bits(838..=838)]
    pub organ_farfisa_preset_1_drawbar_9: bool,
    #[bits(848..=852)]
    pub organ_b3_preset_2_drawbar_1_wheel: RangedU8<31>,
    #[bits(853..=857)]
    pub organ_b3_preset_2_drawbar_1_aftertouch: RangedU8<31>,
    #[bits(858..=862)]
    pub organ_b3_preset_2_drawbar_1_ctrl_pedal: RangedU8<31>,
    #[bits(863..=866)]
    pub organ_b3_preset_2_drawbar_1: RangedU8<15>,
    #[bits(867..=871)]
    pub organ_b3_preset_2_drawbar_2_wheel: RangedU8<31>,
    #[bits(872..=876)]
    pub organ_b3_preset_2_drawbar_2_aftertouch: RangedU8<31>,
    #[bits(877..=881)]
    pub organ_b3_preset_2_drawbar_2_ctrl_pedal: RangedU8<31>,
    #[bits(882..=885)]
    pub organ_b3_preset_2_drawbar_2: RangedU8<15>,
    #[bits(886..=890)]
    pub organ_b3_preset_2_drawbar_3_wheel: RangedU8<31>,
    #[bits(891..=895)]
    pub organ_b3_preset_2_drawbar_3_aftertouch: RangedU8<31>,
    #[bits(896..=900)]
    pub organ_b3_preset_2_drawbar_3_ctrl_pedal: RangedU8<31>,
    #[bits(901..=904)]
    pub organ_farfisa_preset_1_drawbar_4_3: RangedU8<15>,
    #[bits(905..=909)]
    pub organ_b3_preset_2_drawbar_4_wheel: RangedU8<31>,
    #[bits(910..=914)]
    pub organ_b3_preset_2_drawbar_4_aftertouch: RangedU8<31>,
    #[bits(915..=919)]
    pub organ_b3_preset_2_drawbar_4_ctrl_pedal: RangedU8<31>,
    #[bits(920..=923)]
    pub organ_b3_preset_2_drawbar_4: RangedU8<15>,
    #[bits(924..=928)]
    pub organ_b3_preset_2_drawbar_5_wheel: RangedU8<31>,
    #[bits(929..=933)]
    pub organ_b3_preset_2_drawbar_5_aftertouch: RangedU8<31>,
    #[bits(934..=938)]
    pub organ_b3_preset_2_drawbar_5_ctrl_pedal: RangedU8<31>,
    #[bits(939..=942)]
    pub organ_b3_preset_2_drawbar_5: RangedU8<15>,
    #[bits(943..=947)]
    pub organ_b3_preset_2_drawbar_6_wheel: RangedU8<31>,
    #[bits(948..=952)]
    pub organ_b3_preset_2_drawbar_6_aftertouch: RangedU8<31>,
    #[bits(953..=957)]
    pub organ_b3_preset_2_drawbar_6_ctrl_pedal: RangedU8<31>,
    #[bits(958..=961)]
    pub organ_b3_preset_2_drawbar_6: RangedU8<15>,
    #[bits(962..=966)]
    pub organ_b3_preset_2_drawbar_7_wheel: RangedU8<31>,
    #[bits(967..=971)]
    pub organ_b3_preset_2_drawbar_7_aftertouch: RangedU8<31>,
    #[bits(972..=976)]
    pub organ_b3_preset_2_drawbar_7_ctrl_pedal: RangedU8<31>,
    #[bits(977..=980)]
    pub organ_b3_preset_2_drawbar_7: RangedU8<15>,
    #[bits(981..=985)]
    pub organ_b3_preset_2_drawbar_8_wheel: RangedU8<31>,
    #[bits(986..=990)]
    pub organ_b3_preset_2_drawbar_8_aftertouch: RangedU8<31>,
    #[bits(991..=995)]
    pub organ_b3_preset_2_drawbar_8_ctrl_pedal: RangedU8<31>,
    #[bits(996..=999)]
    pub organ_b3_preset_2_drawbar_8: RangedU8<15>,
    #[bits(1000..=1004)]
    pub organ_b3_preset_2_drawbar_9_wheel: RangedU8<31>,
    #[bits(1005..=1009)]
    pub organ_b3_preset_2_drawbar_9_aftertouch: RangedU8<31>,
    #[bits(1010..=1014)]
    pub organ_b3_preset_2_drawbar_9_ctrl_pedal: RangedU8<31>,
    #[bits(1015..=1018)]
    pub organ_b3_preset_2_drawbar_9: RangedU8<15>,
    #[bits(1019..=1019)]
    pub organ_b3_preset_2_vibrato_chorus: bool,
    #[bits(1020..=1020)]
    pub organ_b3_preset_2_percussion: bool,
    #[bits(1032..=1036)]
    pub organ_vox_preset_2_drawbar_1_wheel: RangedU8<31>,
    #[bits(1037..=1041)]
    pub organ_vox_preset_2_drawbar_1_aftertouch: RangedU8<31>,
    #[bits(1042..=1046)]
    pub organ_vox_preset_2_drawbar_1_ctrl_pedal: RangedU8<31>,
    #[bits(1047..=1050)]
    pub organ_vox_preset_2_drawbar_1: RangedU8<15>,
    #[bits(1051..=1055)]
    pub organ_vox_preset_2_drawbar_2_wheel: RangedU8<31>,
    #[bits(1056..=1060)]
    pub organ_vox_preset_2_drawbar_2_aftertouch: RangedU8<31>,
    #[bits(1061..=1065)]
    pub organ_vox_preset_2_drawbar_2_ctrl_pedal: RangedU8<31>,
    #[bits(1066..=1069)]
    pub organ_vox_preset_2_drawbar_2: RangedU8<15>,
    #[bits(1070..=1074)]
    pub organ_vox_preset_2_drawbar_3_wheel: RangedU8<31>,
    #[bits(1075..=1079)]
    pub organ_vox_preset_2_drawbar_3_aftertouch: RangedU8<31>,
    #[bits(1080..=1084)]
    pub organ_vox_preset_2_drawbar_3_ctrl_pedal: RangedU8<31>,
    #[bits(1085..=1088)]
    pub organ_b3_preset_2_drawbar_3: RangedU8<15>,
    #[bits(1089..=1093)]
    pub organ_vox_preset_2_drawbar_4_wheel: RangedU8<31>,
    #[bits(1094..=1098)]
    pub organ_vox_preset_2_drawbar_4_aftertouch: RangedU8<31>,
    #[bits(1099..=1103)]
    pub organ_vox_preset_2_drawbar_4_ctrl_pedal: RangedU8<31>,
    #[bits(1104..=1107)]
    pub organ_vox_preset_2_drawbar_4: RangedU8<15>,
    #[bits(1108..=1112)]
    pub organ_vox_preset_2_drawbar_5_wheel: RangedU8<31>,
    #[bits(1113..=1117)]
    pub organ_vox_preset_2_drawbar_5_aftertouch: RangedU8<31>,
    #[bits(1118..=1122)]
    pub organ_vox_preset_2_drawbar_5_ctrl_pedal: RangedU8<31>,
    #[bits(1123..=1126)]
    pub organ_vox_preset_2_drawbar_5: RangedU8<15>,
    #[bits(1127..=1131)]
    pub organ_vox_preset_2_drawbar_6_wheel: RangedU8<31>,
    #[bits(1132..=1136)]
    pub organ_vox_preset_2_drawbar_6_aftertouch: RangedU8<31>,
    #[bits(1137..=1141)]
    pub organ_vox_preset_2_drawbar_6_ctrl_pedal: RangedU8<31>,
    #[bits(1142..=1145)]
    pub organ_vox_preset_2_drawbar_6: RangedU8<15>,
    #[bits(1146..=1150)]
    pub organ_vox_preset_2_drawbar_7_wheel: RangedU8<31>,
    #[bits(1151..=1155)]
    pub organ_vox_preset_2_drawbar_7_aftertouch: RangedU8<31>,
    #[bits(1156..=1160)]
    pub organ_vox_preset_2_drawbar_7_ctrl_pedal: RangedU8<31>,
    #[bits(1161..=1164)]
    pub organ_vox_preset_2_drawbar_7: RangedU8<15>,
    #[bits(1165..=1169)]
    pub organ_vox_preset_2_drawbar_8_wheel: RangedU8<31>,
    #[bits(1170..=1174)]
    pub organ_vox_preset_2_drawbar_8_aftertouch: RangedU8<31>,
    #[bits(1175..=1179)]
    pub organ_vox_preset_2_drawbar_8_ctrl_pedal: RangedU8<31>,
    #[bits(1180..=1183)]
    pub organ_vox_preset_2_drawbar_8: RangedU8<15>,
    #[bits(1184..=1188)]
    pub organ_vox_preset_2_drawbar_9_wheel: RangedU8<31>,
    #[bits(1189..=1193)]
    pub organ_vox_preset_2_drawbar_9_aftertouch: RangedU8<31>,
    #[bits(1194..=1198)]
    pub organ_vox_preset_2_drawbar_9_ctrl_pedal: RangedU8<31>,
    #[bits(1199..=1202)]
    pub organ_vox_preset_2_drawbar_9: RangedU8<15>,
    #[bits(1216..=1217)]
    pub organ_farfisa_preset_2_drawbar_1_wheel: RangedU8<3>,
    #[bits(1218..=1219)]
    pub organ_farfisa_preset_2_drawbar_1_aftertouch: RangedU8<3>,
    #[bits(1220..=1221)]
    pub organ_farfisa_preset_2_drawbar_1_ctrl_pedal: RangedU8<3>,
    #[bits(1222..=1222)]
    pub organ_farfisa_preset_2_drawbar_1: bool,
    #[bits(1223..=1224)]
    pub organ_farfisa_preset_2_drawbar_2_wheel: RangedU8<3>,
    #[bits(1225..=1226)]
    pub organ_farfisa_preset_2_drawbar_2_aftertouch: RangedU8<3>,
    #[bits(1227..=1228)]
    pub organ_farfisa_preset_2_drawbar_2_ctrl_pedal: RangedU8<3>,
    #[bits(1229..=1229)]
    pub organ_farfisa_preset_2_drawbar_2: bool,
    #[bits(1230..=1231)]
    pub organ_farfisa_preset_2_drawbar_3_wheel: RangedU8<3>,
    #[bits(1232..=1233)]
    pub organ_farfisa_preset_2_drawbar_3_aftertouch: RangedU8<3>,
    #[bits(1234..=1235)]
    pub organ_farfisa_preset_2_drawbar_3_ctrl_pedal: RangedU8<3>,
    #[bits(1236..=1236)]
    pub organ_vox_preset_2_drawbar_3: bool,
    #[bits(1237..=1238)]
    pub organ_farfisa_preset_2_drawbar_4_wheel: RangedU8<3>,
    #[bits(1239..=1240)]
    pub organ_farfisa_preset_2_drawbar_4_aftertouch: RangedU8<3>,
    #[bits(1241..=1242)]
    pub organ_farfisa_preset_2_drawbar_4_ctrl_pedal: RangedU8<3>,
    #[bits(1243..=1243)]
    pub organ_farfisa_preset_2_drawbar_4: bool,
    #[bits(1244..=1245)]
    pub organ_farfisa_preset_2_drawbar_5_wheel: RangedU8<3>,
    #[bits(1246..=1247)]
    pub organ_farfisa_preset_2_drawbar_5_aftertouch: RangedU8<3>,
    #[bits(1248..=1249)]
    pub organ_farfisa_preset_2_drawbar_5_ctrl_pedal: RangedU8<3>,
    #[bits(1250..=1250)]
    pub organ_farfisa_preset_2_drawbar_5: bool,
    #[bits(1251..=1252)]
    pub organ_farfisa_preset_2_drawbar_6_wheel: RangedU8<3>,
    #[bits(1253..=1254)]
    pub organ_farfisa_preset_2_drawbar_6_aftertouch: RangedU8<3>,
    #[bits(1255..=1256)]
    pub organ_farfisa_preset_2_drawbar_6_ctrl_pedal: RangedU8<3>,
    #[bits(1257..=1257)]
    pub organ_farfisa_preset_2_drawbar_6: bool,
    #[bits(1258..=1259)]
    pub organ_farfisa_preset_2_drawbar_7_wheel: RangedU8<3>,
    #[bits(1260..=1261)]
    pub organ_farfisa_preset_2_drawbar_7_aftertouch: RangedU8<3>,
    #[bits(1262..=1263)]
    pub organ_farfisa_preset_2_drawbar_7_ctrl_pedal: RangedU8<3>,
    #[bits(1264..=1264)]
    pub organ_farfisa_preset_2_drawbar_7: bool,
    #[bits(1265..=1266)]
    pub organ_farfisa_preset_2_drawbar_8_wheel: RangedU8<3>,
    #[bits(1267..=1268)]
    pub organ_farfisa_preset_2_drawbar_8_aftertouch: RangedU8<3>,
    #[bits(1269..=1270)]
    pub organ_farfisa_preset_2_drawbar_8_ctrl_pedal: RangedU8<3>,
    #[bits(1271..=1271)]
    pub organ_farfisa_preset_2_drawbar_8: bool,
    #[bits(1272..=1273)]
    pub organ_farfisa_preset_2_drawbar_9_wheel: RangedU8<3>,
    #[bits(1274..=1275)]
    pub organ_farfisa_preset_2_drawbar_9_aftertouch: RangedU8<3>,
    #[bits(1276..=1277)]
    pub organ_farfisa_preset_2_drawbar_9_ctrl_pedal: RangedU8<3>,
    #[bits(1278..=1278)]
    pub organ_farfisa_preset_2_drawbar_9: bool,
    #[bits(1288..=1290)]
    pub piano_type: RangedU8<7>,
    #[bits(1303..=1304)]
    pub piano_clavinet_model: RangedU8<3>,
    #[bits(1305..=1305)]
    pub piano_long_release: bool,
    #[bits(1306..=1306)]
    pub piano_string_resonance: bool,
    #[bits(1307..=1307)]
    pub piano_pedal_noise: bool,
    #[bits(1308..=1309)]
    pub piano_dynamics: RangedU8<3>,
    #[bits(1310..=1311)]
    pub piano_clav_eq_hi: RangedU8<3>,
    #[bits(1312..=1313)]
    pub piano_clav_eq: RangedU8<3>,
    #[bits(1314..=1345)]
    pub piano_sample_id: u32,
    #[bits(1391..=1391)]
    pub synth_arp_on: bool,
    #[bits(1392..=1392)]
    pub synth_arp_master_clock: bool,
    #[bits(1393..=1396)]
    pub synth_arp_master_clock_divisor: RangedU8<15>,
    #[bits(1398..=1404)]
    pub synth_arp_rate: RangedU8<127>,
    #[bits(1405..=1406)]
    pub synth_arp_pattern: RangedU8<3>,
    #[bits(1407..=1408)]
    pub synth_arp_master_range: RangedU8<3>,
    #[bits(1409..=1409)]
    pub synth_lfo_master_clock: bool,
    #[bits(1410..=1413)]
    pub synth_lfo_rate_clock_divisor: RangedU8<15>,
    #[bits(1414..=1414)]
    pub synth_kb_hold: bool,
    #[bits(1432..=1438)]
    pub synth_mod_env_attack: RangedU8<127>,
    #[bits(1439..=1445)]
    pub synth_mod_env_decay: RangedU8<127>,
    #[bits(1446..=1452)]
    pub synth_mod_env_release: RangedU8<127>,
    #[bits(1453..=1453)]
    pub synth_mod_env_velocity: bool,
    #[bits(1454..=1456)]
    pub synth_osc_mode: RangedU8<7>,
    #[bits(1457..=1466)]
    pub synth_osc_waveform: RangedU16<1023>,
    #[bits(1467..=1474)]
    pub synth_shape_wheel: u8,
    #[bits(1475..=1482)]
    pub synth_shape_aftertouch: u8,
    #[bits(1483..=1490)]
    pub synth_shape_ctrl_pedal: u8,
    #[bits(1491..=1497)]
    pub synth_shape: RangedU8<127>,
    #[bits(1498..=1504)]
    pub synth_shape_mod: RangedU8<127>,
    #[bits(1505..=1512)]
    pub synth_shape_detune_wheel: u8,
    #[bits(1513..=1520)]
    pub synth_shape_detune_aftertouch: u8,
    #[bits(1521..=1528)]
    pub synth_shape_detune_ctrl_pedal: u8,
    #[bits(1529..=1535)]
    pub synth_shape_detune: RangedU8<127>,
    #[bits(1536..=1537)]
    pub synth_skip_sample_attack_wheel: RangedU8<3>,
    #[bits(1538..=1539)]
    pub synth_skip_sample_attack_aftertouch: RangedU8<3>,
    #[bits(1540..=1541)]
    pub synth_skip_sample_attack_ctrl_pedal: RangedU8<3>,
    #[bits(1542..=1542)]
    pub synth_skip_sample_attack: bool,
    #[bits(1543..=1550)]
    pub synth_filter_freq_wheel: u8,
    #[bits(1551..=1558)]
    pub synth_filter_freq_aftertouch: u8,
    #[bits(1559..=1566)]
    pub synth_filter_freq_ctrl_pedal: u8,
    #[bits(1567..=1573)]
    pub synth_filter_freq: RangedU8<127>,
    #[bits(1574..=1580)]
    pub synth_filter_resonance: RangedU8<127>,
    #[bits(1581..=1587)]
    pub synth_filter_mod_2: RangedU8<127>,
    #[bits(1588..=1594)]
    pub synth_filter_mod_1: RangedU8<127>,
    #[bits(1595..=1595)]
    pub synth_filter_kb_track: bool,
    #[bits(1596..=1598)]
    pub synth_filter_type: RangedU8<7>,
    #[bits(1599..=1605)]
    pub synth_amp_env_attack: RangedU8<127>,
    #[bits(1606..=1612)]
    pub synth_amp_env_decay: RangedU8<127>,
    #[bits(1613..=1619)]
    pub synth_amp_env_release: RangedU8<127>,
    #[bits(1620..=1620)]
    pub synth_amp_env_velocity: bool,
    #[bits(1621..=1627)]
    pub synth_lfo_rate: RangedU8<127>,
    #[bits(1628..=1629)]
    pub synth_lfo_waveform: RangedU8<3>,
    #[bits(1630..=1661)]
    pub synth_sample_id: u32,
    #[bits(1662..=1668)]
    pub synth_glide_rate: RangedU8<127>,
    #[bits(1669..=1670)]
    pub synth_glide_voice_mode: RangedU8<3>,
    #[bits(1671..=1673)]
    pub synth_unison: RangedU8<7>,
    #[bits(1674..=1676)]
    pub synth_vibrato: RangedU8<7>,
    #[bits(1688..=1689)]
    pub extern_midi_control: RangedU8<3>,
    #[bits(1690..=1696)]
    pub extern_midi_cc_number: RangedU8<127>,
    #[bits(1697..=1704)]
    pub extern_midi_cc_wheel: u8,
    #[bits(1705..=1712)]
    pub extern_midi_cc_aftertouch: u8,
    #[bits(1713..=1720)]
    pub extern_midi_cc_ctrl_pedal: u8,
    #[bits(1721..=1727)]
    pub extern_midi_cc: RangedU8<127>,
    #[bits(1728..=1728)]
    pub extern_midi_cc_on: bool,
    #[bits(1729..=1735)]
    pub extern_midi_bank_select_cc32: RangedU8<127>,
    #[bits(1736..=1736)]
    pub extern_midi_bank_select_cc32_enabled: bool,
    #[bits(1737..=1743)]
    pub extern_midi_bank_select_cc00: RangedU8<127>,
    #[bits(1744..=1744)]
    pub extern_midi_bank_select_cc00_enabled: bool,
    #[bits(1745..=1751)]
    pub extern_midi_program: RangedU8<127>,
    #[bits(1752..=1752)]
    pub extern_midi_program_on: bool,
    #[bits(1753..=1756)]
    pub extern_midi_channel: RangedU8<15>,
    #[bits(1758..=1758)]
    pub extern_midi_channel_type: bool,
    #[bits(1759..=1766)]
    pub extern_volume_wheel: u8,
    #[bits(1767..=1774)]
    pub extern_volume_aftertouch: u8,
    #[bits(1775..=1782)]
    pub extern_volume_ctrl_pedal: u8,
    #[bits(1783..=1789)]
    pub extern_volume: RangedU8<127>,
    #[bits(1790..=1790)]
    pub extern_midi_volume_on: bool,
    #[bits(1791..=1791)]
    pub extern_midi_send_wheel: bool,
    #[bits(1792..=1792)]
    pub extern_midi_send_aftertouch: bool,
    #[bits(1793..=1793)]
    pub extern_midi_send_control_pedal: bool,
    #[bits(1795..=1796)]
    pub extern_midi_velocity_curve: RangedU8<3>,
    #[bits(1797..=1797)]
    pub extern_midi_send_swell: bool,
    #[bits(1816..=1817)]
    pub effect_focus: RangedU8<3>,
    #[bits(1818..=1818)]
    pub effect_1_on: bool,
    #[bits(1819..=1820)]
    pub effect_1_source: RangedU8<3>,
    #[bits(1821..=1823)]
    pub effect_1_type: Effect1Type,
    #[bits(1824..=1824)]
    pub effect_1_master_clock: bool,
    #[bits(1825..=1829)]
    pub effect_1_rate_mst_clock_divisor_wheel: RangedU8<31>,
    #[bits(1830..=1834)]
    pub effect_1_rate_mst_clock_divisor_aftertouch: RangedU8<31>,
    #[bits(1835..=1839)]
    pub effect_1_rate_mst_clock_divisor_ctrl_pedal: RangedU8<31>,
    #[bits(1840..=1843)]
    pub effect_1_rate_mst_clock_divisor: RangedU8<15>,
    #[bits(1844..=1851)]
    pub effect_1_rate_wheel: u8,
    #[bits(1852..=1859)]
    pub effect_1_rate_aftertouch: u8,
    #[bits(1860..=1867)]
    pub effect_1_rate_ctrl_pedal: u8,
    #[bits(1868..=1874)]
    pub effect_1_rate: RangedU8<127>,
    #[bits(1875..=1882)]
    pub effect_1_amount_wheel: u8,
    #[bits(1883..=1890)]
    pub effect_1_amount_aftertouch: u8,
    #[bits(1891..=1898)]
    pub effect_1_amount_ctrl_pedal: u8,
    #[bits(1899..=1905)]
    pub effect_1_amount: RangedU8<127>,
    #[bits(1906..=1906)]
    pub effect_2_on: bool,
    #[bits(1907..=1908)]
    pub effect_2_source: RangedU8<3>,
    #[bits(1909..=1911)]
    pub effect_2_type: Effect2Type,
    #[bits(1912..=1912)]
    pub effect_2_master_clock: bool,
    #[bits(1913..=1917)]
    pub effect_2_rate_mst_clock_divisor_wheel: RangedU8<31>,
    #[bits(1918..=1922)]
    pub effect_2_rate_mst_clock_divisor_aftertouch: RangedU8<31>,
    #[bits(1923..=1927)]
    pub effect_2_rate_mst_clock_divisor_ctrl_pedal: RangedU8<31>,
    #[bits(1928..=1931)]
    pub effect_2_rate_mst_clock_divisor: RangedU8<15>,
    #[bits(1932..=1939)]
    pub effect_2_rate_wheel: u8,
    #[bits(1940..=1947)]
    pub effect_2_rate_aftertouch: u8,
    #[bits(1948..=1955)]
    pub effect_2_rate_ctrl_pedal: u8,
    #[bits(1956..=1962)]
    pub effect_2_rate: RangedU8<127>,
    #[bits(1963..=1970)]
    pub effect_2_amount_wheel: u8,
    #[bits(1971..=1978)]
    pub effect_2_amount_aftertouch: u8,
    #[bits(1979..=1986)]
    pub effect_2_amount_ctrl_pedal: u8,
    #[bits(1987..=1993)]
    pub effect_2_amount: RangedU8<127>,
    #[bits(1994..=1994)]
    pub delay_on: bool,
    #[bits(1995..=1996)]
    pub delay_source: RangedU8<3>,
    #[bits(1997..=1997)]
    pub delay_ping_pong: bool,
    #[bits(1998..=1998)]
    pub delay_master_clock: bool,
    #[bits(1999..=2003)]
    pub delay_tempo_master_clock_divisor_wheel_o_delay_on: RangedU8<31>,
    #[bits(2004..=2008)]
    pub delay_tempo_master_clock_divisor_aftertouch: RangedU8<31>,
    #[bits(2009..=2013)]
    pub delay_tempo_master_clock_divisor_ctrl_pedal: RangedU8<31>,
    #[bits(2014..=2017)]
    pub delay_tempo_master_clock_divisor: RangedU8<15>,
    #[bits(2018..=2030)]
    pub delay_tempo_master_clock_divisor_wheel: RangedU16<8191>,
    #[bits(2031..=2043)]
    pub delay_tempo_aftertouch: RangedU16<8191>,
    #[bits(2044..=2056)]
    pub delay_tempo_ctrl_pedal: RangedU16<8191>,
    #[bits(2057..=2068)]
    pub delay_tempo: RangedU16<4095>,
    #[bits(2069..=2076)]
    pub delay_tempo_wheel: u8,
    #[bits(2077..=2084)]
    pub delay_amount_aftertouch: u8,
    #[bits(2085..=2092)]
    pub delay_amount_ctrl_pedal: u8,
    #[bits(2093..=2099)]
    pub delay_amount: RangedU8<127>,
    #[bits(2100..=2106)]
    pub delay_feedback: RangedU8<127>,
    #[bits(2107..=2107)]
    pub amp_sim_eq_on: bool,
    #[bits(2108..=2109)]
    pub amp_sim_eq_source: RangedU8<3>,
    #[bits(2110..=2111)]
    pub amp_type: RangedU8<3>,
    #[bits(2112..=2118)]
    pub amp_sim_drive: RangedU8<127>,
    #[bits(2119..=2125)]
    pub eq_treble: RangedU8<127>,
    #[bits(2126..=2132)]
    pub eq_mid: RangedU8<127>,
    #[bits(2133..=2139)]
    pub eq_bass: RangedU8<127>,
    #[bits(2140..=2146)]
    pub eq_mid_flt_freq: RangedU8<127>,
}

impl Program {
    /// Whether any split is active.
    pub fn split_enabled(&self) -> bool {
        self.split_two_zones || self.split_three_zones
    }
}

/// The category byte the header's `aux` word carries; the three bytes above it
/// are zero on every corpus specimen.
pub fn category(header: &Header) -> ProgramCategory {
    use crate::bits::Packed;
    ProgramCategory::from_bits((header.aux & 0xff) as u64).expect("decoding is total")
}

/// The `(bank, location)` pair from the header, uninterpreted: bank 0..=3,
/// location 0..=99 on current exports. Not validated — see the Stage 3's note on
/// out-of-range locations in old files.
pub fn location(file: &Cbin<Program>) -> (u16, u16) {
    file.header.slot()
}

pub fn read_from(reader: &mut (impl Read + Seek)) -> Result<Cbin<Program>, Error> {
    let file: Cbin<Program> = cbin::read(reader, FORMAT)?;
    crate::formats::known_version(FORMAT, file.header.version, KNOWN_VERSIONS)?;
    Ok(file)
}

sparse_enum!(
    /// From the `ns2-organ-kb-zone` table in the Stage byte-map docs.
    OrganKbZone, 3, {
        0 => Lo, "LO";
        1 => LoUp, "LO UP";
        2 => Up, "UP";
        3 => UpHi, "UP HI";
        4 => Hi, "HI";
        5 => LoUpHi, "LO UP HI";
    }
);

sparse_enum!(
    /// From the `ns2-piano-kb-zone` table in the Stage byte-map docs.
    PianoKbZone, 3, {
        0 => Lo, "LO";
        1 => LoUp, "LO UP";
        2 => Up, "UP";
        3 => UpHi, "UP HI";
        4 => Hi, "HI";
        5 => LoUpHi, "LO UP HI";
    }
);

sparse_enum!(
    /// From the `ns2-synth-kb-zone` table in the Stage byte-map docs.
    SynthKbZone, 3, {
        0 => Lo, "LO";
        1 => LoUp, "LO UP";
        2 => Up, "UP";
        3 => UpHi, "UP HI";
        4 => Hi, "HI";
        5 => LoUpHi, "LO UP HI";
    }
);
