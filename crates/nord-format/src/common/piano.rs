use crate::common::container::Container;
use crate::error::{Error, ParseError};
use std::fmt;
use std::fmt::Debug;
use std::io::{Read, Seek, Write};

pub const FORMAT: &str = "npno";

/// A piano library file.
///
/// Nothing below the CBIN header is mapped, so this is the container and its body held
/// verbatim: enough to identify a file and hand it back unchanged, and no more.
pub struct Piano {
    container: Container,
}

impl Piano {
    pub fn new() -> Piano {
        Piano {
            container: Container::new(FORMAT, 0, 0, Vec::new()),
        }
    }

    pub fn read_from(reader: &mut (impl Read + Seek)) -> Result<Piano, Error> {
        let container = Container::read_all(reader)?;
        if container.header.tag != FORMAT {
            return Err(ParseError::WrongFormat {
                expected: FORMAT,
                got: container.header.tag,
            }
            .into());
        }
        Ok(Piano { container })
    }

    pub fn write_to(&self, writer: &mut (impl Write + Seek)) -> Result<(), Error> {
        self.container.write_to(writer)
    }
}

impl Default for Piano {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Piano {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("common::Piano")
            .field("format", &self.container.header.tag)
            .field("body", &self.container.body.len())
            .finish()
    }
}
