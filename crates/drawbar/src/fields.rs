//! The registry plumbing under the document: which bodies have fields, how a change is
//! applied, and which control a field asks for.
//!
//! Field paths, values and refusals all come from `nord-format`, so a field becomes
//! editable by being declared and nothing here can fall behind the library.

use std::io::Cursor;
use std::ops::Range;

use nord_format::cbin::Cbin;
use nord_format::fields::{Field, FieldError};
use nord_format::formats::{ne5, ns2, ns3};
use nord_format::{Entity, Live, Program, Settings};

use crate::drawbar_widget;

/// The bodies the document drives: one vocabulary — `path = value` — over each.
trait Editable {
    fn fields(&self) -> Vec<Field>;
    fn set_field(&mut self, path: &str, value: &str) -> Result<(), FieldError>;
}

macro_rules! editable {
    ($body:ty) => {
        impl Editable for Cbin<$body> {
            fn fields(&self) -> Vec<Field> {
                self.body.fields()
            }
            fn set_field(&mut self, path: &str, value: &str) -> Result<(), FieldError> {
                self.body.set_field(path, value)
            }
        }
    };
}

editable!(ne5::Program);
editable!(ne5::Settings);
editable!(ns2::Program);
editable!(ns3::Program);

/// The registry-backed body an entity holds, if it has one.
///
/// ⚠️ Kept in step with [`body_mut`] below — a variant in one and not the other is a
/// body that lists its fields and refuses to set them, or the reverse.
fn body(entity: &Entity) -> Option<&dyn Editable> {
    match entity {
        Entity::Program(Program::Electro5(f)) => Some(f),
        Entity::Program(Program::Stage2(f)) => Some(f),
        Entity::Program(Program::Stage3(f)) => Some(f),
        // The live buffer is the program body under another tag, so the fields are
        // identical.
        Entity::Live(Live::Electro5(f)) => Some(f),
        Entity::Live(Live::Stage2(f)) => Some(f),
        Entity::Live(Live::Stage3(f)) => Some(f),
        Entity::Settings(Settings::Electro5(f)) => Some(f),
        _ => None,
    }
}

fn body_mut(entity: &mut Entity) -> Option<&mut dyn Editable> {
    match entity {
        Entity::Program(Program::Electro5(f)) => Some(f),
        Entity::Program(Program::Stage2(f)) => Some(f),
        Entity::Program(Program::Stage3(f)) => Some(f),
        Entity::Live(Live::Electro5(f)) => Some(f),
        Entity::Live(Live::Stage2(f)) => Some(f),
        Entity::Live(Live::Stage3(f)) => Some(f),
        Entity::Settings(Settings::Electro5(f)) => Some(f),
        _ => None,
    }
}

/// Every registered field's current value, for a body that has a registry.
pub fn fields_of(entity: &Entity) -> Option<Vec<Field>> {
    body(entity).map(|body| body.fields())
}

/// Whether the body is an Electro 5 panel — the one the document knows section by
/// section. Everything else with a registry falls back to a plain field list.
pub fn is_electro5_panel(entity: &Entity) -> bool {
    matches!(
        entity,
        Entity::Program(Program::Electro5(_)) | Entity::Live(Live::Electro5(_))
    )
}

pub fn is_electro5_settings(entity: &Entity) -> bool {
    matches!(entity, Entity::Settings(Settings::Electro5(_)))
}

/// Apply every set to a fresh decode of `bytes` and re-encode.
///
/// Every change lands before anything is encoded, so a value the field cannot hold
/// cannot leave a half-edited body behind. Applying the sets together is what lets a
/// control that owns two fields — the transpose pair — move both or neither.
pub fn apply(bytes: &[u8], sets: &[(String, String)]) -> Result<(Vec<Field>, Vec<u8>), String> {
    let mut entity =
        nord_format::from_stream(&mut Cursor::new(bytes)).map_err(|e| e.to_string())?;
    {
        let body = body_mut(&mut entity).ok_or("this entity has no field registry")?;
        for (path, value) in sets {
            body.set_field(path, value).map_err(|e| e.to_string())?;
        }
    }
    let fields = body(&entity)
        .ok_or("this entity has no field registry")?
        .fields();
    let out = nord_format::to_bytes(&entity).map_err(|e| e.to_string())?;
    Ok((fields, out))
}

/// Longest legal-value list that stays a menu. Past it, a run of consecutive integers is
/// a slider instead.
const CHOICE_MAX: usize = 24;

/// Which control a field asks for, from its type alone.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Control {
    Toggle,
    Choice,
    Number {
        min: i64,
        max: i64,
    },
    /// A nine-nibble organ register.
    Register,
    /// Too wide to enumerate, so the stored bits are its only spelling.
    Stored,
}

impl Control {
    pub fn of(field: &Field, legal: &[String]) -> Control {
        if drawbar_widget::is_register(&field.path, field.spec.width) {
            return Control::Register;
        }
        // A field past the enumerable ceiling lists no values at all.
        if legal.is_empty() {
            return Control::Stored;
        }
        if legal.len() == 2 && legal[0] == "false" && legal[1] == "true" {
            return Control::Toggle;
        }
        if legal.len() > CHOICE_MAX {
            if let Some((min, max)) = contiguous(legal) {
                return Control::Number { min, max };
            }
        }
        Control::Choice
    }
}

/// The range a legal-value list covers, when every value is an integer and none is
/// missing. A gapped set stays a menu — a slider over it would stop on values the field
/// refuses.
fn contiguous(legal: &[String]) -> Option<(i64, i64)> {
    let mut values = Vec::with_capacity(legal.len());
    for value in legal {
        values.push(value.trim_start_matches('+').parse::<i64>().ok()?);
    }
    let min = *values.iter().min()?;
    let max = *values.iter().max()?;
    (max.checked_sub(min)? + 1 == values.len() as i64).then_some((min, max))
}

/// One byte that moved.
pub struct DiffRow {
    pub at: usize,
    pub before: u8,
    pub after: u8,
    /// `  (body crc32)` where the byte is bookkeeping rather than an edit.
    pub note: &'static str,
}

/// Where a CBIN file keeps its checksum and what to call it, or `None` for bytes that
/// are not a CBIN file.
///
/// ⚠️ The two generations put it in different places, and a type-0 file's `0x18` is body
/// data — annotating it as the type-1 crc32 would label a real edit as bookkeeping.
fn checksum_bytes(file: &[u8]) -> Option<(Range<usize>, &'static str)> {
    if file.len() < 8 || &file[0..4] != nord_format::cbin::MAGIC {
        return None;
    }
    match u32::from_le_bytes(file[4..8].try_into().ok()?) {
        0 => Some((file.len() - 2..file.len(), "  (file crc16)")),
        1 => Some((0x18..0x1c, "  (body crc32)")),
        _ => None,
    }
}

/// The bytes that moved.
///
/// The checksum moves with any body change; those rows are annotated so they do not read
/// as a second unexplained edit. A length change is not a diff at all — nothing here can
/// pair the bytes up — so it comes back empty.
pub fn byte_diff(before: &[u8], after: &[u8]) -> Vec<DiffRow> {
    if before.len() != after.len() {
        return Vec::new();
    }
    let checksum = checksum_bytes(after);
    before
        .iter()
        .zip(after)
        .enumerate()
        .filter(|(_, (b, a))| b != a)
        .map(|(at, (&b, &a))| DiffRow {
            at,
            before: b,
            after: a,
            note: match &checksum {
                Some((range, label)) if range.contains(&at) => label,
                _ => "",
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program() -> Vec<u8> {
        let entity = Entity::Program(Program::Electro5(ne5::program::new(
            (0, 0).try_into().unwrap(),
        )));
        nord_format::to_bytes(&entity).unwrap()
    }

    #[test]
    fn a_set_changes_the_field_it_names_and_nothing_else() {
        let bytes = program();
        let (before, _) = apply(&bytes, &[]).unwrap();
        let (after, edited) = apply(&bytes, &[("center_panel.gain".into(), "96".into())]).unwrap();

        let moved: Vec<&str> = before
            .iter()
            .zip(&after)
            .filter(|(b, a)| b.display != a.display)
            .map(|(_, a)| a.path.as_str())
            .collect();
        assert_eq!(moved, ["center_panel.gain"]);
        assert_eq!(edited.len(), bytes.len());
    }

    /// A value the field cannot hold is refused before anything is encoded, and the
    /// message names what it does accept.
    #[test]
    fn an_out_of_range_value_is_refused_by_the_library() {
        let err = apply(&program(), &[("center_panel.gain".into(), "200".into())])
            .err()
            .expect("200 is not a gain");
        assert!(err.contains("not a value of gain"), "{err}");
        assert!(err.contains("0 .. 127"), "{err}");
    }

    /// One refused half of a two-field control must not leave the other half applied.
    #[test]
    fn a_refusal_anywhere_in_a_batch_applies_none_of_it() {
        let bytes = program();
        let sets = [
            ("center_panel.transpose_enabled".into(), "true".into()),
            ("center_panel.transpose".into(), "99".into()),
        ];
        assert!(apply(&bytes, &sets).is_err());
        let (fields, _) = apply(&bytes, &[]).unwrap();
        let enabled = fields
            .iter()
            .find(|f| f.path == "center_panel.transpose_enabled")
            .unwrap();
        assert_eq!(enabled.value, "false");
    }

    /// The crc moves with any body change; the row has to say so, or it reads as a
    /// second edit nobody made.
    #[test]
    fn the_checksum_bytes_are_annotated_as_bookkeeping() {
        let bytes = program();
        let (_, edited) = apply(&bytes, &[("center_panel.gain".into(), "96".into())]).unwrap();
        let diff = byte_diff(&bytes, &edited);
        assert!(!diff.is_empty());
        // A fresh program is type-1, so the crc32 sits at 0x18..0x1c.
        let annotated: Vec<usize> = diff
            .iter()
            .filter(|row| row.note.contains("crc32"))
            .map(|row| row.at)
            .collect();
        assert!(annotated.iter().all(|at| (0x18..0x1c).contains(at)));
        assert!(!annotated.is_empty(), "the crc32 must have moved");
        assert!(
            diff.iter().any(|row| row.note.is_empty()),
            "the edit itself must show as an unannotated byte",
        );
    }

    /// A gapped legal set must stay a menu: a slider over it would stop on values the
    /// field refuses.
    #[test]
    fn only_a_gapless_run_of_integers_becomes_a_slider() {
        let full: Vec<String> = (0..128).map(|n| n.to_string()).collect();
        assert_eq!(contiguous(&full), Some((0, 127)));

        let gapped: Vec<String> = vec!["0".into(), "1".into(), "9".into()];
        assert_eq!(contiguous(&gapped), None);

        let named: Vec<String> = vec!["Organ".into(), "Piano".into()];
        assert_eq!(contiguous(&named), None);
    }

    /// The nine-nibble register is the drawbar widget's, and nothing else is.
    #[test]
    fn a_register_field_picks_the_drawbar_control() {
        let bytes = program();
        let (fields, _) = apply(&bytes, &[]).unwrap();
        let register = fields
            .iter()
            .find(|f| f.path == "organ_panel.b3_preset1_drawbars")
            .expect("a program has a b3 preset 1 register");
        assert_eq!(
            Control::of(register, &(register.spec.legal)()),
            Control::Register
        );

        let gain = fields
            .iter()
            .find(|f| f.path == "center_panel.gain")
            .expect("a program has a gain");
        assert_eq!(
            Control::of(gain, &(gain.spec.legal)()),
            Control::Number { min: 0, max: 127 }
        );
    }

    /// A drawbar pulled in the widget writes back exactly what the field reads out, so
    /// parking a bar where it already was is not a change.
    #[test]
    fn a_register_round_trips_through_the_widgets_spelling() {
        let bytes = program();
        let (fields, _) = apply(&bytes, &[]).unwrap();
        let register = fields
            .iter()
            .find(|f| f.path == "organ_panel.vox_preset1_drawbars")
            .unwrap();
        let bits = drawbar_widget::parse(&register.value).unwrap();
        let spelled = drawbar_widget::spell(drawbar_widget::bits(drawbar_widget::bars(bits)));
        assert_eq!(spelled, register.value);

        let (after, _) = apply(&bytes, &[(register.path.clone(), "0x888800000".into())]).unwrap();
        let edited = after.iter().find(|f| f.path == register.path).unwrap();
        assert_eq!(
            drawbar_widget::bars(drawbar_widget::parse(&edited.value).unwrap()),
            [8, 8, 8, 8, 0, 0, 0, 0, 0]
        );
    }
}
