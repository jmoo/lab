//! Scratch: mutate a decoded panel field and write the program out.
//!
//! The fields exercised here live in `CenterPanel`'s `settings3` and `settings2`. On
//! master, `settings3` had a `bw(ignore)` on every decoded field and a verbatim backing
//! word, so a write to `transpose` was silently discarded; `settings2` had a `bw(calc)`
//! and did land. Setting both at once discriminates between the two behaviours: if only
//! `settings2` reaches the file, the transpose *light* comes on while the value stays
//! where it was.
//!
//!     cargo run -p nord-format --example set_field -- <in> <out> transpose <-6..6>
//!     cargo run -p nord-format --example set_field -- <in> <out> organ <b3|vox|farfisa|pipe>

use nord_format::electro5::OrganType;
use nord_format::{electro5, Entity};
use std::io::Cursor;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, input, output, field, value] = args.as_slice() else {
        eprintln!("usage: set_field <in> <out> <transpose|organ> <value>");
        std::process::exit(2);
    };

    let Entity::Program(nord_format::Program::Electro5(mut program)) =
        nord_format::from_path(input).expect("read input")
    else {
        panic!("not an electro5 program");
    };

    let before;
    let after;
    match field.as_str() {
        "transpose" => {
            let c = &mut program.schema.center_panel;
            before = format!(
                "transpose {:+} (enabled {})",
                c.transpose.inner(),
                c.transpose_enabled
            );
            c.transpose = value
                .parse::<i8>()
                .expect("-6..6")
                .try_into()
                .expect("in range");
            c.transpose_enabled = true;
            after = format!(
                "transpose {:+} (enabled {})",
                c.transpose.inner(),
                c.transpose_enabled
            );
        }
        "organ" => {
            let want = match value.as_str() {
                "b3" => OrganType::B3,
                "b3bass" => OrganType::B3Bass,
                "pipe" => OrganType::Pipe,
                "vox" => OrganType::Vox,
                "farfisa" => OrganType::Farfisa,
                other => panic!("unknown organ: {other}"),
            };
            let c = &mut program.schema.center_panel;
            before = c.organ_type.to_string();
            c.organ_type = want;
            after = want.to_string();
        }
        other => panic!("unknown field: {other}"),
    }

    let mut bytes = Vec::new();
    program
        .write_to(&mut Cursor::new(&mut bytes))
        .expect("write");
    assert_eq!(bytes.len(), electro5::program::FILE_LEN);
    std::fs::write(output, &bytes).expect("write file");

    let original = std::fs::read(input).expect("re-read");
    let diff: Vec<usize> = original
        .iter()
        .zip(&bytes)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i)
        .collect();

    println!("{before}  ->  {after}");
    println!("bytes changed: {}", diff.len());
    for i in &diff {
        let word = match i {
            0x18..=0x1b => "crc",
            0x2e..=0x2f => "centre settings",
            0x30 => "centre settings2",
            0x31..=0x34 => "centre settings3",
            _ => "",
        };
        println!(
            "  {i:#04x}  {:#04x} -> {:#04x}  {word}",
            original[*i], bytes[*i]
        );
    }
}
