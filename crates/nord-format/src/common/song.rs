use crate::common::bank::Location;
use std::fmt::Debug;

/// A set list body: the program slots a song steps through, in play order.
///
/// Generic over the slot spaces because the count and both address spaces are the
/// format's: the Electro 5 names four programs from a four-bank song space, and other
/// models will differ. Everything else a set list file carries — tag, version,
/// generation, its own slot — lives on the container.
#[derive(Debug)]
pub struct Setlist<const PROGRAM_COUNT: usize, ProgramLocation>
where
    ProgramLocation: Location,
{
    programs: [ProgramLocation; PROGRAM_COUNT],
}

impl<const C: usize, P> Setlist<C, P>
where
    P: Location,
{
    pub fn new(programs: [P; C]) -> Setlist<C, P> {
        Setlist { programs }
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
