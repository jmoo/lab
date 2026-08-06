//! Body layouts as data.
//!
//! `#[bitbody]` generates an implementation of [`BodyLayout`] alongside the codec,
//! so a body's byte map exists once in the source and is readable at runtime —
//! for generated documentation, for `nord inspect`, for anything that wants to
//! answer "which bytes does this field own" without re-stating the layout.
//!
//! A [`Segment`] is one contiguous byte range of the body. Panel segments chain
//! down into their [`FieldSpec`]s, so the full map — file offset to bit — is one
//! walk: body segments, then each panel's fields.
//!
//! [`FieldSpec`]: crate::panel::FieldSpec

/// One contiguous byte range of a body, half-open and body-relative.
///
/// For the file offset a hex dump shows, add the container's body start — `0x2c`
/// on a type-1 file, `0x18` on a type-0.
#[derive(Clone)]
pub struct Segment {
    /// The field's name in the body struct.
    pub name: &'static str,
    /// First byte of the segment.
    pub start: usize,
    /// One past the last byte.
    pub end: usize,
    /// The field's Rust type, as written.
    pub ty: &'static str,
    pub kind: SegmentKind,
}

#[derive(Clone)]
pub enum SegmentKind {
    /// Bytes kept verbatim through a re-encode: padding and unmapped ranges.
    Verbatim,
    /// An unsigned integer.
    Uint { big_endian: bool },
    /// A bit-packed panel; `field_specs` lists its fields.
    Panel {
        field_specs: fn() -> Vec<crate::panel::FieldSpec>,
    },
}

/// A body whose byte map is declared once, by `#[bitbody]`.
pub trait BodyLayout {
    /// The segments, in file order, covering every body byte exactly once —
    /// the macro refuses to compile a layout with a gap or an overlap.
    fn layout() -> &'static [Segment];
}

impl std::fmt::Debug for Segment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match &self.kind {
            SegmentKind::Verbatim => "verbatim".to_string(),
            SegmentKind::Uint { big_endian: true } => "uint be".to_string(),
            SegmentKind::Uint { big_endian: false } => "uint le".to_string(),
            SegmentKind::Panel { field_specs } => {
                format!("panel ({} fields)", field_specs().len())
            }
        };
        write!(
            f,
            "{:#04x}..{:#04x} {} ({}, {kind})",
            self.start, self.end, self.name, self.ty
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cbin::{self, Cbin, Header};
    use nord_bits_derive::{bitbody, bitpanel};
    use std::io::Cursor;

    #[bitpanel(2)]
    #[derive(Default)]
    struct TestPanel {
        #[bits(0..=0)]
        flag: bool,
        #[bits(4..=11)]
        level: u8,
    }

    /// Every segment kind the macro knows: a big-endian word, a verbatim pad, a
    /// panel, and a little-endian tail.
    #[bitbody(10)]
    struct TestBody {
        #[at(0x00..0x02, be)]
        word: u16,
        #[at(0x02..0x04)]
        pad: [u8; 2],
        #[at(0x04..0x06)]
        panel: TestPanel,
        #[at(0x06..0x0a, le)]
        tail: u32,
    }

    fn body() -> TestBody {
        TestBody {
            word: 0x0102,
            pad: [0xaa, 0xbb],
            panel: TestPanel::try_from([0x80, 0x40]).unwrap(),
            tail: 0xdead_beef,
        }
    }

    /// The one declaration serves both directions, byte for byte.
    #[test]
    fn the_codec_is_the_declaration() {
        let raw = <[u8; 10]>::from(&body());
        assert_eq!(
            raw,
            [0x01, 0x02, 0xaa, 0xbb, 0x80, 0x40, 0xef, 0xbe, 0xad, 0xde],
        );
        let back = TestBody::try_from(raw).unwrap();
        assert_eq!(<[u8; 10]>::from(&back), raw);
        assert_eq!(back.word, 0x0102);
        assert_eq!(back.tail, 0xdead_beef);
        assert!(back.panel.flag);
    }

    /// The generated `Body` impl carries a bitbody through the container whole:
    /// fixed length enforced, checksum stamped, both generations.
    #[test]
    fn a_bitbody_rides_the_container() {
        for generation in [cbin::Generation::V1, cbin::Generation::V0] {
            let mut header = Header::new("tstb", (2, 5), 7);
            header.generation = generation;
            let file = Cbin {
                header,
                body: body(),
            };
            let mut bytes = Cursor::new(Vec::new());
            file.write_to(&mut bytes).unwrap();
            let mut bytes = Cursor::new(bytes.into_inner());
            let back: Cbin<TestBody> = cbin::read(&mut bytes, "tstb").unwrap();
            assert_eq!(back.header.slot(), (2, 5));
            assert_eq!(<[u8; 10]>::from(&back.body), <[u8; 10]>::from(&body()));
        }
    }

    /// The layout is data: every byte accounted for, in order, and a panel
    /// segment chains down into its own field specs.
    #[test]
    fn the_layout_is_readable_as_data() {
        let segments = TestBody::layout();
        let names: Vec<_> = segments.iter().map(|s| s.name).collect();
        assert_eq!(names, ["word", "pad", "panel", "tail"]);

        let mut cursor = 0;
        for s in segments {
            assert_eq!(s.start, cursor, "{}: gap or overlap", s.name);
            cursor = s.end;
        }
        assert_eq!(cursor, 10, "the segments do not cover the body");

        let SegmentKind::Panel { field_specs } = &segments[2].kind else {
            panic!("`panel` is not a panel segment: {:?}", segments[2]);
        };
        let fields: Vec<_> = field_specs().into_iter().map(|f| f.name).collect();
        assert_eq!(fields, ["flag", "level"]);
        assert!(matches!(
            segments[0].kind,
            SegmentKind::Uint { big_endian: true }
        ));
        assert!(matches!(segments[1].kind, SegmentKind::Verbatim));
    }

    /// Verbatim segments survive a round trip but stay out of `Debug`.
    #[test]
    fn pads_round_trip_but_do_not_print() {
        let printed = format!("{:?}", body());
        assert!(printed.contains("word"), "{printed}");
        assert!(!printed.contains("pad"), "{printed}");
    }
}
