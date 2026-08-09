//! Which banks have been read, and which are still owed.
//!
//! The instrument reports how many objects a class holds but never how the panel
//! divides them into banks, so a class is walked one bank at a time until the device
//! stops answering. That answer arrives as a *short* bank — fewer slots than were asked
//! for, because the walk hit status 3 — and that is what ends the walk.

use std::collections::{HashMap, VecDeque};

use nord_usb::wire::Status;
use nord_usb::ObjectClass;

use super::slots_per_bank;

/// ⚠️ A ceiling on a walk whose end the device alone decides. Nothing on the wire says a
/// class cannot hold more banks than this; the cap is here so an instrument that never
/// answers "out of range" cannot spin the queue forever.
const MAX_BANKS: u32 = 32;

/// How far through a class the background read has got.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Progress {
    /// Banks read so far.
    pub done: u32,
    /// Banks expected, where the inventory said enough to work it out.
    pub total: Option<u32>,
    /// More banks are queued for this class.
    pub running: bool,
}

/// The queue of banks still to read, and what has been read already.
#[derive(Default)]
pub struct Scan {
    queue: VecDeque<(ObjectClass, u32)>,
    /// Keyed by the raw class number, because [`ObjectClass`] is not `Hash`.
    progress: HashMap<u32, Progress>,
}

impl Scan {
    /// Read `class` from its first bank, discarding anything already queued for it.
    pub fn start(&mut self, class: ObjectClass, total: Option<u32>) {
        self.queue.retain(|(queued, _)| *queued != class);
        self.queue.push_back((class, 1));
        self.progress.insert(
            class.to_raw(),
            Progress {
                done: 0,
                total,
                running: true,
            },
        );
    }

    /// Take the next bank to ask about off the queue.
    pub fn take(&mut self) -> Option<(ObjectClass, u32)> {
        self.queue.pop_front()
    }

    /// Read one bank again on its own, without restarting the walk through the class.
    ///
    /// What a mutation owes: only the banks it touched can have changed names.
    pub fn again(&mut self, class: ObjectClass, bank: u32) {
        if !self.queue.contains(&(class, bank)) {
            self.queue.push_back((class, bank));
        }
    }

    /// Record a bank that came back. `full` means the device answered every slot that
    /// was asked for, which is the only sign that another bank may follow it.
    ///
    /// The walk only continues from the bank it was itself up to, so a one-off re-read
    /// of an already-visited bank does not set the rest of the class going again.
    pub fn scanned(&mut self, class: ObjectClass, bank: u32, full: bool) {
        let progress = self.progress.entry(class.to_raw()).or_default();
        let walking = bank == progress.done + 1;
        progress.done = progress.done.max(bank);
        let more =
            walking && full && bank < MAX_BANKS && progress.total.is_none_or(|total| bank < total);
        if walking {
            progress.running = more;
        }
        if more {
            self.queue.push_back((class, bank + 1));
        }
    }

    /// The class stopped answering, so nothing more is owed for it.
    pub fn stopped(&mut self, class: ObjectClass) {
        self.queue.retain(|(queued, _)| *queued != class);
        if let Some(progress) = self.progress.get_mut(&class.to_raw()) {
            progress.running = false;
        }
    }

    pub fn progress(&self, class: ObjectClass) -> Option<Progress> {
        self.progress.get(&class.to_raw()).copied()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.progress.clear();
    }
}

/// How many banks a class is expected to have, when the inventory says enough to tell.
///
/// ⚠️ The count is derived from the class's slot total and [`slots_per_bank`], both of
/// which are the Electro 5's divisions rather than anything the wire reports. `None`
/// means the walk runs until the device refuses a bank.
pub fn bank_count(class: ObjectClass, inventory: &[Status]) -> Option<u32> {
    match class {
        // Both are singletons: one bank, however the rest of the classes are divided.
        ObjectClass::Live | ObjectClass::Settings => Some(1),
        _ => inventory
            .iter()
            .find(|status| status.class == class)?
            .slots()
            .map(|slots| slots.div_ceil(slots_per_bank(class))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(class: ObjectClass, count: u32, free: u32, used: u32) -> Status {
        Status {
            class,
            count,
            free,
            used,
        }
    }

    /// A bank that answered every slot may have a successor, so the walk goes on.
    #[test]
    fn a_full_bank_queues_the_one_after_it() {
        let mut scan = Scan::default();
        scan.start(ObjectClass::Program, None);
        assert_eq!(scan.take(), Some((ObjectClass::Program, 1)));
        scan.scanned(ObjectClass::Program, 1, true);
        assert_eq!(scan.take(), Some((ObjectClass::Program, 2)));
        assert!(scan.progress(ObjectClass::Program).unwrap().running);
    }

    /// A short bank is the device saying the class ends here.
    #[test]
    fn a_short_bank_ends_the_walk() {
        let mut scan = Scan::default();
        scan.start(ObjectClass::Sample, None);
        scan.take();
        scan.scanned(ObjectClass::Sample, 1, false);
        assert_eq!(scan.take(), None);
        let progress = scan.progress(ObjectClass::Sample).unwrap();
        assert!(!progress.running);
        assert_eq!(progress.done, 1);
    }

    /// A known total stops the walk without spending an operation on a bank that
    /// cannot exist.
    #[test]
    fn a_known_total_stops_the_walk_at_the_last_bank() {
        let mut scan = Scan::default();
        scan.start(ObjectClass::Program, Some(2));
        scan.take();
        scan.scanned(ObjectClass::Program, 1, true);
        assert_eq!(scan.take(), Some((ObjectClass::Program, 2)));
        scan.scanned(ObjectClass::Program, 2, true);
        assert_eq!(scan.take(), None);
        assert!(!scan.progress(ObjectClass::Program).unwrap().running);
    }

    /// Asking for a class again starts it over rather than appending to a walk that is
    /// already part-way through — the names it read are the ones being replaced.
    #[test]
    fn reading_a_class_again_starts_it_over() {
        let mut scan = Scan::default();
        scan.start(ObjectClass::Program, Some(8));
        scan.take();
        scan.scanned(ObjectClass::Program, 1, true);
        scan.start(ObjectClass::Program, Some(8));
        assert_eq!(scan.take(), Some((ObjectClass::Program, 1)));
        assert_eq!(scan.take(), None);
        assert_eq!(scan.progress(ObjectClass::Program).unwrap().done, 0);
    }

    /// A bank re-read after a mutation is one operation, not the tail of the class.
    #[test]
    fn re_reading_one_bank_does_not_restart_the_walk() {
        let mut scan = Scan::default();
        scan.start(ObjectClass::Program, Some(8));
        for bank in 1..=8 {
            scan.take();
            scan.scanned(ObjectClass::Program, bank, true);
        }
        assert_eq!(scan.take(), None);

        scan.again(ObjectClass::Program, 7);
        assert_eq!(scan.take(), Some((ObjectClass::Program, 7)));
        scan.scanned(ObjectClass::Program, 7, true);
        assert_eq!(scan.take(), None);
    }

    /// One class stopping leaves the others queued.
    #[test]
    fn stopping_one_class_leaves_the_others_queued() {
        let mut scan = Scan::default();
        scan.start(ObjectClass::Program, None);
        scan.start(ObjectClass::SetList, None);
        scan.stopped(ObjectClass::Program);
        assert_eq!(scan.take(), Some((ObjectClass::SetList, 1)));
        assert!(!scan.progress(ObjectClass::Program).unwrap().running);
    }

    /// An Electro 5 reports 400 program slots, which is its eight banks of fifty.
    #[test]
    fn the_bank_count_comes_out_of_the_slot_total() {
        // 141 blocks an item, 400 items: the numbers a real Electro 5 answers with.
        let inventory = [status(ObjectClass::Program, 379, 2961, 53439)];
        assert_eq!(
            bank_count(ObjectClass::Program, &inventory),
            Some(400_u32.div_ceil(50))
        );
    }

    /// Variable-size classes divide into nothing, so their walk is open-ended; the
    /// singletons are one bank whatever the inventory says.
    #[test]
    fn a_class_the_inventory_cannot_divide_has_no_expected_bank_count() {
        let inventory = [status(ObjectClass::Sample, 7, 100, 333)];
        assert_eq!(bank_count(ObjectClass::Sample, &inventory), None);
        assert_eq!(bank_count(ObjectClass::Piano, &inventory), None);
        assert_eq!(bank_count(ObjectClass::Live, &inventory), Some(1));
        assert_eq!(bank_count(ObjectClass::Settings, &[]), Some(1));
    }
}
