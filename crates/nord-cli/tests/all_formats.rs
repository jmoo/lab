//! Every format `nord-format` reads, pushed through the real binary: one file
//! per CBIN tag plus the non-CBIN carriers, each synthesized through the
//! library's own writers — no corpus needed. `nord inspect` must name every
//! one and `nord verify` must round-trip every one, so a format the library
//! gains but the CLI cannot at least identify fails here.

use nord_format::cbin::{Cbin, Header, RawBody};
use nord_format::formats::{
    nc2, nc2d, nd2, nd3, ne3, ne4, ne5, ne6, ne7, ng2, nl4, nla1, no3, np, np2, np3, np4, np5,
    npip, npno, ns2, ns3, ns4, nsclassic, nsmp, nw, nw2,
};
use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;

/// `(tag, body length, a version the reader accepts)` for every CBIN format.
/// Stubs accept any version; the decoded formats gate on their known ones.
fn formats() -> Vec<(&'static str, u64, u32)> {
    vec![
        (nc2::program::FORMAT, nc2::program::BODY_LEN, 100),
        (nc2::settings::FORMAT, nc2::settings::BODY_LEN, 100),
        (nc2d::program::FORMAT, nc2d::program::BODY_LEN, 100),
        (nc2d::settings::FORMAT, nc2d::settings::BODY_LEN, 100),
        (nd2::program::FORMAT, nd2::program::BODY_LEN, 3),
        (nd3::kit::FORMAT, nd3::kit::BODY_LEN, 1),
        (ne3::program::FORMAT, ne3::program::BODY_LEN, 101),
        (ne3::organ_preset::FORMAT, ne3::organ_preset::BODY_LEN, 100),
        (ne4::program::FORMAT, ne4::program::BODY_LEN, 103),
        (ne4::live::FORMAT, ne4::live::BODY_LEN, 103),
        (ne4::settings::FORMAT, ne4::settings::BODY_LEN, 100),
        (ne5::song::FORMAT, ne5::song::BODY_LEN as u64, 0),
        (ne5::settings::FORMAT, ne5::settings::BODY_LEN as u64, 0),
        (ne6::program::FORMAT, ne6::program::BODY_LEN, 204),
        (ne6::live::FORMAT, ne6::live::BODY_LEN, 204),
        (ne6::settings::FORMAT, ne6::settings::BODY_LEN, 200),
        (ne7::program::FORMAT, ne7::program::BODY_LEN, 110),
        (ne7::live::FORMAT, ne7::live::BODY_LEN, 110),
        (ne7::settings::FORMAT, ne7::settings::BODY_LEN, 301),
        (ng2::program::FORMAT, ng2::program::BODY_LEN, 102),
        (ng2::live::FORMAT, ng2::live::BODY_LEN, 102),
        (ng2::settings::FORMAT, ng2::settings::BODY_LEN, 102),
        (nl4::program::FORMAT, nl4::program::BODY_LEN, 7),
        (nl4::performance::FORMAT, nl4::performance::BODY_LEN, 7),
        (nl4::settings::FORMAT, nl4::settings::BODY_LEN, 3),
        (nla1::program::FORMAT, nla1::program::BODY_LEN, 6),
        (nla1::performance::FORMAT, nla1::performance::BODY_LEN, 6),
        (nla1::settings::FORMAT, nla1::settings::BODY_LEN, 0),
        (no3::program::FORMAT, no3::program::BODY_LEN, 200),
        (no3::settings::FORMAT, no3::settings::BODY_LEN, 200),
        (np::program::FORMAT, np::program::BODY_LEN, 103),
        (np::live::FORMAT, np::live::BODY_LEN, 104),
        (np::settings::FORMAT, np::settings::BODY_LEN, 100),
        (np2::program::FORMAT, np2::program::BODY_LEN, 1),
        (np2::live::FORMAT, np2::live::BODY_LEN, 0),
        (np2::settings::FORMAT, np2::settings::BODY_LEN, 1),
        (np3::program::FORMAT, np3::program::BODY_LEN, 4),
        (np3::live::FORMAT, np3::live::BODY_LEN, 4),
        (np3::settings::FORMAT, np3::settings::BODY_LEN, 0),
        (np4::program::FORMAT, np4::program::BODY_LEN, 100),
        (np4::live::FORMAT, np4::live::BODY_LEN, 100),
        (np4::settings::FORMAT, np4::settings::BODY_LEN, 100),
        (np5::program::FORMAT, np5::program::BODY_LEN, 101),
        (np5::live::FORMAT, np5::live::BODY_LEN, 101),
        (np5::settings::FORMAT, np5::settings::BODY_LEN, 100),
        (npip::pipe_library::FORMAT, 64, 100),
        (npno::FORMAT, 64, 0),
        (ns2::program::FORMAT, ns2::program::BODY_LEN as u64, 6),
        (ns2::live::FORMAT, ns2::program::BODY_LEN as u64, 6),
        (ns2::synth::FORMAT, ns2::synth::BODY_LEN, 6),
        (ns2::settings::FORMAT, ns2::settings::BODY_LEN, 4),
        (ns3::program::FORMAT, ns3::program::BODY_LEN as u64, 304),
        (ns3::live::FORMAT, ns3::program::BODY_LEN as u64, 304),
        (ns3::song::FORMAT, ns3::song::BODY_LEN, 300),
        (ns3::synth::FORMAT, ns3::synth::BODY_LEN, 300),
        (ns3::settings::FORMAT, ns3::settings::BODY_LEN, 300),
        (ns4::program::FORMAT, ns4::program::BODY_LEN, 313),
        (ns4::live::FORMAT, ns4::live::BODY_LEN, 313),
        (ns4::synth::FORMAT, ns4::synth::BODY_LEN, 208),
        (ns4::piano_preset::FORMAT, ns4::piano_preset::BODY_LEN, 203),
        (ns4::organ_preset::FORMAT, ns4::organ_preset::BODY_LEN, 205),
        (ns4::settings::FORMAT, ns4::settings::BODY_LEN, 106),
        (
            nsclassic::program::FORMAT,
            nsclassic::program::BODY_LEN,
            316,
        ),
        (nsclassic::synth::FORMAT, nsclassic::synth::BODY_LEN, 100),
        (nsclassic::piano_library::FORMAT, 64, 210),
        // ≥300 is the nsmp3/nsmp4 generation, kept verbatim; a v2 body needs
        // real section chains a synthesized file cannot fake.
        (nsmp::FORMAT, 64, 300),
        (nw::program::FORMAT, nw::program::BODY_LEN, 8),
        (nw::settings::FORMAT, nw::settings::BODY_LEN, 5),
        (nw2::program::FORMAT, nw2::program::BODY_LEN, 301),
        (nw2::live::FORMAT, nw2::live::BODY_LEN, 301),
        (nw2::settings::FORMAT, nw2::settings::BODY_LEN, 300),
    ]
}

fn synthesize(tag: &str, body_len: u64, version: u32) -> Vec<u8> {
    let file = Cbin {
        header: Header::new(tag, (0, 0), version),
        body: RawBody(vec![0u8; body_len as usize]),
    };
    let mut out = Cursor::new(Vec::new());
    file.write_to(&mut out).unwrap();
    out.into_inner()
}

/// The two formats whose decode refuses an all-zero body (an octave shift of
/// raw 0 is out of range), built from the library's default program instead —
/// the same constructor `nord program edit` starts a blank file from.
fn ne5_programs() -> Vec<(&'static str, Vec<u8>)> {
    let emit = |file: nord_format::cbin::Cbin<ne5::Program>| {
        let mut out = Cursor::new(Vec::new());
        file.write_to(&mut out).unwrap();
        out.into_inner()
    };
    vec![
        (
            ne5::program::FORMAT,
            emit(ne5::program::new(
                ne5::program::Location::new(0, 0).unwrap(),
            )),
        ),
        (
            ne5::live::FORMAT,
            emit(ne5::live::new(ne5::live::Location::new(0, 0).unwrap())),
        ),
    ]
}

/// The non-CBIN carriers, minimal but valid to their readers.
fn carriers() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("lead2-bank.syx", vec![0xf0, 0x33, 0x0f, 0x04, 0x00, 0xf7]),
        ("lead-bank.mid", b"MThd\0\0\0\x06\0\0\0\x01\0\x60".to_vec()),
        ("electro2-library.cn3", b"CNE3\x2c\x01\0\0\0\0\0\0".to_vec()),
    ]
}

fn nord(verb: &str, paths: &[PathBuf]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_nord"))
        .arg(verb)
        .args(paths)
        .output()
        .expect("running the nord binary")
}

#[test]
fn every_format_inspects_and_verifies() {
    let dir = std::env::temp_dir().join(format!("nord-all-formats-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let mut paths = Vec::new();
    for (tag, body_len, version) in formats() {
        // Tags may end in NUL, which a filename cannot hold; dispatch is by
        // content, so the name is only for the failure message.
        let path = dir.join(format!("{}.nord", tag.trim_end_matches('\0')));
        std::fs::write(&path, synthesize(tag, body_len, version)).unwrap();
        paths.push(path);
    }
    for (tag, bytes) in ne5_programs() {
        let path = dir.join(format!("{tag}.nord"));
        std::fs::write(&path, bytes).unwrap();
        paths.push(path);
    }
    for (name, bytes) in carriers() {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        paths.push(path);
    }

    let out = nord("inspect", &paths);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "inspect failed:\n{}{}",
        stdout,
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        stdout.matches("type:").count(),
        paths.len(),
        "every file gets a type line:\n{stdout}"
    );

    let out = nord("verify", &paths);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "verify failed:\n{}{}",
        stdout,
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        stdout.lines().filter(|l| l.starts_with("ok")).count(),
        paths.len(),
        "every file round-trips:\n{stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}
