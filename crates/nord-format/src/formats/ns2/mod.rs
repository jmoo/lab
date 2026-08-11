//! Nord Stage 2 and 2 EX (`.ns2p`, `.ns2l`, `.ns2s`, `.ns2y`).
//!
//! The program body decodes in full, **both slots**: the byte maps document one,
//! and the body holds two — 23 bytes of globals then two 249-byte slot blocks,
//! the second repeating the first. Slot A's fields are unprefixed and slot B's
//! carry `slot_b_`. The remaining formats are container-verified stubs; the synth
//! file (`ns2s`) is slot A's synth block, located but not yet declared. ⚠️ `s` is a synth patch here
//! (a song on the Stage 3) and `y` is the settings (a synth patch on the 3/4).

use super::raw::raw_format;

pub mod program;
pub use program::Program;

pub mod live {
    //! The live buffer (`.ns2l`): same body as a program, under its own tag.

    use super::program::{self, Program};
    use crate::cbin::{self, Cbin};
    use crate::error::Error;
    use std::io::{Read, Seek};

    pub const FORMAT: &str = "ns2l";

    pub fn read_from(reader: &mut (impl Read + Seek)) -> Result<Cbin<Program>, Error> {
        let file: Cbin<Program> = cbin::read(reader, FORMAT)?;
        crate::formats::known_version(FORMAT, file.header.version, program::KNOWN_VERSIONS)?;
        Ok(file)
    }
}

raw_format!(
    /// Synth patches (`.ns2s`).
    synth,
    "ns2s",
    34
);
raw_format!(
    /// Settings (`.ns2y`).
    settings,
    "ns2y",
    32
);
