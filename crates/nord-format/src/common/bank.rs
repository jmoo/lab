use std::fmt::Debug;
use std::hash::Hash;

/// A two-axis slot address, as the instruments display them (`bank:slot`).
///
/// Implemented by [`crate::types::RangedU16Pair`], which bounds each axis to one
/// format's space. Files carry no names — a slot's name lives on the instrument — so
/// an address is all a decoded entity knows about where it sits.
pub trait Location:
    Debug + Clone + Copy + PartialEq + Eq + Hash + TryFrom<u16> + TryFrom<(u16, u16)>
{
    fn inner(&self) -> (u16, u16);
    fn as_u16(&self) -> u16;
    fn x(&self) -> u16;
    fn y(&self) -> u16;
}
