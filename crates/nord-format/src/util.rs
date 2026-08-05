use binrw::BinReaderExt;

use crate::common::container;

use crate::error::{Error, ParseError};

pub enum FileType {
    Cbin,
    Xml,
    Zip,
}

impl FileType {
    pub fn as_str(&self) -> &str {
        match self {
            FileType::Cbin => "cbin",
            FileType::Xml => "xml",
            FileType::Zip => "zip",
        }
    }
}

pub struct Peek {
    pub format: String,
    pub file_type: FileType,
}

/**
 * Peek at the first byte of a file to determine its type.
 */
pub fn peek(reader: &mut impl BinReaderExt) -> Result<Peek, Error> {
    let mut head = [0u8; 1];
    if let Err(e) = reader.read_exact(&mut head) {
        return Err(e.into());
    }
    let head = head[0];

    reader.seek(std::io::SeekFrom::Start(0))?;

    let result = match head {
        0x50 => Ok(Peek {
            format: String::from("unknown"),
            file_type: FileType::Zip,
        }),

        0x3c => Ok(Peek {
            format: String::from("unknown"),
            file_type: FileType::Xml,
        }),

        // The magic and the tag, which is as far as dispatch needs to look. The
        // generation and the checksum are the container's to check, once a format module
        // has said which file this is.
        0x43 => {
            let mut head = [0u8; 12];
            reader.read_exact(&mut head)?;
            container::header_type(&head)?;
            Ok(Peek {
                format: String::from_utf8_lossy(&head[8..12]).into_owned(),
                file_type: FileType::Cbin,
            })
        }

        _ => Err(ParseError::UnknownFormat(format!("first_byte = {:0x}", head)).into()),
    };

    reader.seek(std::io::SeekFrom::Start(0))?;

    result
}
