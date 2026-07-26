//! `nord device` — talk to an attached instrument over USB.
//!
//! Only read-only operations live here for now. `status` sends one request per object
//! class and reads counters back; nothing on the instrument changes, which makes it
//! the safe way to prove the transport, framing and session layers work end to end
//! against real hardware.

use std::path::PathBuf;

use nord_usb::op;
use nord_usb::transport::Transport;
use nord_usb::wire::Status;

/// Where to get the exchange from.
pub enum Source {
    /// A real instrument over USB.
    Usb,
    /// A recorded exchange. Lets the whole path be demonstrated with no hardware —
    /// and is how this command is exercised under Wine, qemu and in CI.
    Replay(PathBuf),
}

pub fn run(source: Source, json: bool) -> Result<(), String> {
    let report = match source {
        Source::Usb => {
            let mut transport =
                nord_usb::transport::UsbTransport::open_first().map_err(|e| e.to_string())?;
            collect(&mut transport)?
        }
        Source::Replay(path) => {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            let mut transport = nord_usb::ReplayTransport::from_script(&text)
                .map_err(|e| e.to_string())?
                .lenient();
            collect(&mut transport)?
        }
    };

    if json {
        print_json(&report);
    } else {
        print_table(&report);
    }
    Ok(())
}

fn collect<T: Transport>(transport: &mut T) -> Result<Vec<Status>, String> {
    nord_usb::block_on(op::inventory(transport)).map_err(|e| e.to_string())
}

fn print_table(report: &[Status]) {
    if report.is_empty() {
        println!("no classes answered");
        return;
    }
    println!("{:<10} {:>20} {:>7}  {}", "class", "used", "full", "of");
    let mut any_variable = false;
    for s in report {
        // Fixed-size classes are far clearer as slots than as raw blocks: programs
        // report 400, which is exactly the instrument's 8 banks x 50.
        let (used, of) = match s.slots() {
            Some(slots) => (
                format!("{} / {} slots", s.count, slots),
                format!("{} blocks each", s.blocks_per_item().unwrap_or(0)),
            ),
            None => {
                any_variable = true;
                (format!("{} / {} blocks", s.used, s.total()), format!("{} items", s.count))
            }
        };
        println!("{:<10} {:>20} {:>6.1}%  {}", s.class.label(), used, s.used_percent(), of);
    }
    if any_variable {
        // Only worth saying for the classes where the number is genuinely opaque.
        println!("\n(blocks are a device-internal unit, not bytes)");
    }
}

fn print_json(report: &[Status]) {
    println!("[");
    for (i, s) in report.iter().enumerate() {
        let comma = if i + 1 == report.len() { "" } else { "," };
        println!(
            "  {{\"class\": \"{}\", \"code\": {}, \"items\": {}, \"used\": {}, \"free\": {}, \"capacity\": {}}}{comma}",
            s.class.label(),
            s.class.to_raw(),
            s.count,
            s.used,
            s.free,
            s.total(),
        );
    }
    println!("]");
}
