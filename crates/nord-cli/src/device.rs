//! `nord device` — talk to an attached instrument over USB.
//!
//! Read-only queries (`status`, `read`, `deps`) and the non-destructive `select` need
//! no confirmation; the mutating actions (`write`, `move`, `delete`, `rename`,
//! `duplicate`) each describe what they will touch and then refuse to proceed without
//! `--yes`. `status` is the gentlest end-to-end check: it sends one query per object
//! class and reads counters back, changing nothing on the instrument.

use std::path::PathBuf;

use nord_usb::op;
use nord_usb::transport::Transport;
use nord_usb::wire::{Location, Status};
use nord_usb::{op as usb_op, ObjectClass, Session};

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

/// Parse a slot as the instrument displays it: `8-14` is bank 8, slot 14.
pub fn parse_location(s: &str) -> Result<Location, String> {
    let (b, l) = s
        .split_once('-')
        .ok_or_else(|| format!("expected BANK-SLOT (e.g. 7-4), got {s:?}"))?;
    let bank: u32 = b.trim().parse().map_err(|_| format!("bad bank {b:?}"))?;
    let slot: u32 = l.trim().parse().map_err(|_| format!("bad slot {l:?}"))?;
    if bank == 0 || slot == 0 {
        return Err("banks and slots are numbered from 1, as shown on the instrument".into());
    }
    Ok(Location::from_user(bank, slot))
}

fn open_usb() -> Result<nord_usb::transport::UsbTransport, String> {
    nord_usb::transport::UsbTransport::open_first().map_err(|e| e.to_string())
}

/// Read one entity off the instrument. Read-only.
///
/// `class` selects the object type (4 programs, 5 set lists, …). `raw` writes the wire
/// body verbatim instead of wrapping it in a CBIN header — essential for formats whose
/// header layout is not yet known, where wrapping would fabricate a wrong file.
pub fn read(at: Location, out: Option<PathBuf>, class: u32, raw: bool) -> Result<(), String> {
    let class = ObjectClass::from_raw(class);
    let mut t = open_usb()?;
    let (info, file) = nord_usb::block_on(async {
        let mut s = Session::open(&mut t, class).await?;
        let info = usb_op::info(&mut s, at).await?;
        let file = if raw {
            usb_op::read_body(&mut s, at).await?
        } else {
            usb_op::read_program(&mut s, at).await?
        };
        s.commit().await?;
        Ok::<_, nord_usb::Error>((info, file))
    })
    .map_err(|e| e.to_string())?;
    eprintln!("  class={:?} format={:?} body_len={}", class, info.format, info.body_len);

    let path = out.unwrap_or_else(|| {
        // Default to the slot name, which is what the corpus convention keys on.
        let stem = if info.name.is_empty() { "entity".into() } else { info.name.clone() };
        let ext = if raw { "body".to_string() } else { info.format.clone() };
        PathBuf::from(format!("{stem}.{ext}"))
    });
    std::fs::write(&path, &file).map_err(|e| format!("{}: {e}", path.display()))?;
    eprintln!("read {} ({} bytes) -> {}", info.name, file.len(), path.display());
    Ok(())
}

/// Write a `.ne5p` into a slot, overwriting it.
pub fn write(path: PathBuf, at: Location, confirmed: bool) -> Result<(), String> {
    let file = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    // Fail before touching the device if the file is not what it claims to be.
    nord_usb::envelope::unwrap(&file).map_err(|e| e.to_string())?;

    let mut t = open_usb()?;

    // Show what is about to be destroyed before destroying it. Reading the slot
    // first costs one round trip and turns a silent overwrite into an informed one.
    let existing = nord_usb::block_on(async {
        let mut s = Session::open(&mut t, ObjectClass::Program).await?;
        let info = usb_op::info(&mut s, at).await?;
        s.commit().await?;
        Ok::<_, nord_usb::Error>(info)
    })
    .map_err(|e| e.to_string())?;

    eprintln!(
        "about to overwrite bank {} slot {} (currently {:?}) with {}",
        existing.location.bank + 1,
        existing.location.slot + 1,
        existing.name,
        path.display()
    );
    if !confirmed {
        return Err("refusing to write without --yes (back up first: `nord device read`)".into());
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0);

    nord_usb::block_on(async {
        let mut s = Session::open(&mut t, ObjectClass::Program).await?.allow_destructive_writes();
        let r = usb_op::write_program(&mut s, at, &file, timestamp).await;
        // Close the transaction either way; leaving it half-open is worse than the
        // original error.
        let closed = s.commit().await;
        r.and(closed)
    })
    .map_err(|e| e.to_string())?;

    eprintln!("wrote {} -> bank {} slot {}", path.display(), at.bank + 1, at.slot + 1);
    Ok(())
}

/// One-indexed `bank N slot M`, matching the instrument's own labels.
fn shown(at: Location) -> String {
    format!("bank {} slot {}", at.bank + 1, at.slot + 1)
}

/// Read one slot's name/format in a throwaway read-only session — used to show what a
/// mutation is about to affect before it happens.
fn peek(t: &mut nord_usb::transport::UsbTransport, class: ObjectClass, at: Location) -> Result<String, String> {
    nord_usb::block_on(async {
        let mut s = Session::open(t, class).await?;
        let info = usb_op::info(&mut s, at).await?;
        s.commit().await?;
        Ok::<_, nord_usb::Error>(info.name)
    })
    .map_err(|e| e.to_string())
}

/// Refuse a destructive op unless `--yes` was given, after describing what it touches.
fn require_yes(confirmed: bool) -> Result<(), String> {
    if confirmed {
        Ok(())
    } else {
        Err("refusing to modify the device without --yes (back up first: `nord device read`)".into())
    }
}

/// Move an object from one slot to another. Destructive; requires `--yes`.
pub fn move_object(from: Location, to: Location, class: u32, confirmed: bool) -> Result<(), String> {
    let class = ObjectClass::from_raw(class);
    let mut t = open_usb()?;
    let name = peek(&mut t, class, from)?;
    eprintln!("moving {:?} from {} to {}", name, shown(from), shown(to));
    require_yes(confirmed)?;
    nord_usb::block_on(async {
        let mut s = Session::open(&mut t, class).await?.allow_destructive_writes();
        let r = usb_op::move_object(&mut s, from, to).await;
        r.and(s.commit().await)
    })
    .map_err(|e| e.to_string())?;
    eprintln!("moved {} -> {}", shown(from), shown(to));
    Ok(())
}

/// Delete one or more slots. Destructive; requires `--yes`. All items run in one
/// session, exactly as NSM batches a multi-delete.
pub fn delete(slots: &[Location], class: u32, confirmed: bool) -> Result<(), String> {
    let class = ObjectClass::from_raw(class);
    let mut t = open_usb()?;
    for &at in slots {
        let name = peek(&mut t, class, at)?;
        eprintln!("deleting {:?} at {}", name, shown(at));
    }
    require_yes(confirmed)?;
    nord_usb::block_on(async {
        let mut s = Session::open(&mut t, class).await?.allow_destructive_writes();
        let mut r = Ok(());
        for &at in slots {
            r = usb_op::delete(&mut s, at).await;
            if r.is_err() {
                break;
            }
        }
        r.and(s.commit().await)
    })
    .map_err(|e| e.to_string())?;
    eprintln!("deleted {} item(s)", slots.len());
    Ok(())
}

/// Rename the object in a slot. Destructive; requires `--yes`.
pub fn rename(at: Location, name: String, class: u32, confirmed: bool) -> Result<(), String> {
    let class = ObjectClass::from_raw(class);
    let mut t = open_usb()?;
    let old = peek(&mut t, class, at)?;
    eprintln!("renaming {} from {:?} to {:?}", shown(at), old, name);
    require_yes(confirmed)?;
    nord_usb::block_on(async {
        let mut s = Session::open(&mut t, class).await?.allow_destructive_writes();
        let r = usb_op::rename(&mut s, at, &name).await;
        r.and(s.commit().await)
    })
    .map_err(|e| e.to_string())?;
    eprintln!("renamed {} -> {:?}", shown(at), name);
    Ok(())
}

/// Duplicate an object into another slot (a device-internal deep copy). Destructive;
/// requires `--yes`.
pub fn duplicate(from: Location, to: Location, class: u32, confirmed: bool) -> Result<(), String> {
    let class = ObjectClass::from_raw(class);
    let mut t = open_usb()?;
    let name = peek(&mut t, class, from)?;
    eprintln!("duplicating {:?} from {} to {}", name, shown(from), shown(to));
    require_yes(confirmed)?;
    nord_usb::block_on(async {
        let mut s = Session::open(&mut t, class).await?.allow_destructive_writes();
        let r = usb_op::duplicate(&mut s, from, to).await;
        r.and(s.commit().await)
    })
    .map_err(|e| e.to_string())?;
    eprintln!("duplicated {} -> {}", shown(from), shown(to));
    Ok(())
}

/// Load an object live on the instrument (double-click in NSM). Non-destructive, so no
/// confirmation is needed.
pub fn select(at: Location, class: u32) -> Result<(), String> {
    let class = ObjectClass::from_raw(class);
    let mut t = open_usb()?;
    nord_usb::block_on(async {
        let mut s = Session::open(&mut t, class).await?;
        let r = usb_op::select(&mut s, at).await;
        let closed = s.commit().await;
        r.and(closed)
    })
    .map_err(|e| e.to_string())?;
    eprintln!("selected {} on the instrument", shown(at));
    Ok(())
}

/// List the piano/sample library objects an entity depends on. Read-only.
pub fn deps(at: Location, class: u32) -> Result<(), String> {
    let class = ObjectClass::from_raw(class);
    let mut t = open_usb()?;
    let deps = nord_usb::block_on(async {
        let mut s = Session::open(&mut t, class).await?;
        let deps = usb_op::dependencies(&mut s, at).await?;
        s.commit().await?;
        Ok::<_, nord_usb::Error>(deps)
    })
    .map_err(|e| e.to_string())?;

    if deps.is_empty() {
        println!("{} has no dependencies", shown(at));
        return Ok(());
    }
    println!("{:<8} {:<10} name", "class", "id");
    for d in &deps {
        let loc = d.location.map(shown).unwrap_or_default();
        println!("{:<8} {:08x}   {} {}", d.class.label(), d.id, d.name, loc);
    }
    Ok(())
}
