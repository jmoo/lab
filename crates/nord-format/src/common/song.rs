use crate::common::bank::{Item, Location};
use crate::common::container;
use std::fmt::Debug;

#[derive(Debug)]
pub struct Song<const PROGRAM_COUNT: usize, SongLocation, ProgramLocation>
where
    SongLocation: Location,
    ProgramLocation: Location,
{
    name: Option<String>,
    location: SongLocation,
    programs: [ProgramLocation; PROGRAM_COUNT],
    /// The container this song arrived in. A song rebuilds its file on write, so
    /// everything the header holds has to survive here: the eight factory demo songs
    /// are version 0 where everything user-written is 1, and the two generations are
    /// [`container::SIZE_DELTA`] bytes apart.
    ///
    /// ⚠️ Its `tag` is the format module's to stamp at write — a song is generic over
    /// its slot space and does not know which format it is.
    header: container::Header,
}

impl<const C: usize, S, P> Song<C, S, P>
where
    S: Location,
    P: Location,
{
    pub fn new(location: S, programs: [P; C]) -> Song<C, S, P> {
        Song {
            name: None,
            location,
            programs,
            header: container::Header::new("", Self::DEFAULT_VERSION),
        }
    }

    /// What a newly authored song is written as. Reading a file overwrites this with
    /// whatever the file carried.
    pub const DEFAULT_VERSION: u32 = 1;

    /// The container the song was read from, for a format module to write it back with.
    pub fn header(&self) -> &container::Header {
        &self.header
    }

    pub fn set_header(&mut self, header: container::Header) {
        self.header = header;
    }

    /// Container schema version — see the field docs.
    pub fn version(&self) -> u32 {
        self.header.version
    }

    pub fn set_version(&mut self, version: u32) {
        self.header.version = version;
    }

    /// CBIN header generation — see the field docs.
    pub fn header_type(&self) -> u32 {
        self.header.header_type
    }

    pub fn set_header_type(&mut self, header_type: u32) {
        self.header.header_type = header_type;
    }

    pub fn get(&self, slot: u16) -> P {
        self.programs[slot as usize]
    }

    pub fn set(&mut self, slot: u16, location: P) {
        self.programs[slot as usize] = location;
    }

    pub fn programs(&self) -> &[P; C] {
        &self.programs
    }
}

impl<const C: usize, S, P> Item<S> for Song<C, S, P>
where
    S: Location,
    P: Location,
{
    fn name(&self) -> Option<String> {
        self.name.clone()
    }

    fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    fn location(&self) -> S {
        self.location
    }

    fn set_location(&mut self, location: S) {
        self.location = location;
    }
}
