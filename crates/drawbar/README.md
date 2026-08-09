# drawbar

An [egui](https://github.com/emilk/egui) app over
[`nord-format`](../nord-format) and [`nord-usb`](../nord-usb) — everything
[`nord-cli`](../nord-cli) can do that is worth a window, reachable from a browser
tab or a desktop one. Same job as the CLI: **dogfood the libraries**, not ship a
product. The engineer's verbs stay in the CLI.

It is named for the nine-drawbar organ register widget in its editor.

> [!CAUTION]
> # ⚠️ 🧱 USE AT YOUR OWN RISK — THIS TOOL CAN BRICK YOUR DEVICE 🧱 ⚠️
>
> Until this project reaches 1.0, **do not use this tool to speak to a real Nord
> device** unless you are using it to actively develop these libraries.
>
> **I am not responsible for what you do with this tool.**

## What this build does

The window is a file browser over two places your sounds live:

| region | what it holds |
|---|---|
| **Sidebar** (left) | **This computer** — dropped or opened files, copies pulled off the instrument, fresh defaults — and the **instrument**, whose folders (Programs, Set lists, Samples, Pianos, Live, Settings) fill in by themselves once it is attached |
| **Tabs** (centre) | one document per thing you opened. Double-click anything in the sidebar to open it; something on the instrument is copied here first |
| **Status strip** (bottom) | one line about what just happened, and a spinner while something is running. Click it for the full activity log, protocol detail and all |

### This computer

Files arrive by drag-and-drop or **Open…**, and leave through **Save to disk…**
on a row's context menu. Every file is decoded and immediately re-encoded to
check the bytes come back identical; a file that fails to decode still gets a
row, because reporting a bad file is the point of opening it. **New** makes a
fresh program, live or settings document.

### The instrument

**Connect** opens the device (a chooser in the browser, the first attached
Clavia natively) and reads what it holds. From there every bank of every folder
is read in the background, filling names into the tree as they arrive — the
folder heading says how far it has got. Nothing asks for a bank number. **Read
again** re-walks one folder; anything you change is re-read on its own, and a
change made on the panel while the app is attached drops every cached name and
reads them again.

Empty slots are rows, not absences: they are places something can be dropped.
Pianos are listed and nothing more — a piano library is hundreds of megabytes,
and the folder offers no way to pull one down.

### Moving things

Drag between the two places, or within one:

- **instrument → This computer** copies it here.
- **This computer → a slot** sends it. An occupied slot asks once — *Replace
  “Squabble B” in Programs 7:4 with “Africa Split”?* — and an empty one just
  does it.
- **slot → slot, inside one folder** rearranges them. The instrument swaps the
  two, so nothing is lost and nothing asks.

A target that cannot take what you are dragging does not light up; drop on it
anyway and the status strip says why.

Right-click a row for Open, Copy to this computer / Save to disk…, Load on
instrument, Rename, Duplicate, Delete… and Remove from list. Click the name of a
row that is already selected — or press F2 — to rename it in place; Enter
commits, Escape leaves it alone. Renaming on the instrument happens straight
away, because renaming it back is its own undo.

Progress is painted on the *instrument's own display* by the operations
themselves — there is no host-side progress callback, so the app shows a spinner
and does not invent one.

### Safety

- **Read-only until you say otherwise.** There is no armed mode: a destructive
  session exists only for the single operation the app released.
- **Replacing and deleting ask first, by name.** Nothing else does — a move
  loses nothing and a rename undoes itself.
- **Move is a swap, not an overwrite** — the destination's occupant ends up in
  the source slot, byte-identical. Confirmed on hardware.
- **A put is a delete followed by a write** — the instrument refuses to overwrite
  in place. The occupant is read into memory first and written back if the write
  fails; if the restore fails too, its bytes land on This computer as a rescued
  entity rather than being lost.
- **Live and settings refuse a write.** Whether either survives a delete of its
  own class is unconfirmed on hardware, so an edit of either stops at a file.
- **Every session closes, including on the error path.** An abandoned transaction
  strands the instrument on its progress screen with no way out but a power
  cycle.
- **One operation at a time.** What you asked for always goes ahead of the
  background read of the tree, so a click never waits on it.

### The document

A tab holds a working copy. Its header says where it came from, and something copied
off the instrument offers **Send to Programs 7:4** to put it back through the flow
above. **Revert** goes back to the bytes the tab opened with — the only undo there is —
and **Save to disk…** writes the file.

Looking at a document and changing it are the same thing. There is no Apply: a control
you move is set on the working copy that instant, the bytes are re-encoded and
re-checked, and the tab picks up a dot. The sections are the ones on the instrument's
own front panel — Organ, Piano, Sample, Effects, EQ, Keyboard & split — and a settings
document is divided the way the instrument's menus are: System, MIDI, Sound, At
power-on.

**A document shows what the keyboard would be showing.**

- The organ section carries the model picker and **only the selected model's**
  registrations: two nine-drawbar presets for the B3 with its vibrato and percussion,
  the Vox's bars and vibrato, the Farfisa's registers (which the instrument reads as
  on/off tabs at 5 and above), the pipe organ's bars and nothing else. The preset the
  instrument is playing is marked, and clicking the other one switches it.
- In **b3+bass**, preset 1 is the bass manual: two drawbars, in their own fields. The
  nine nibbles they shadow hold stale leftovers and are not shown at all.
- A section no part is playing starts collapsed rather than disappearing — the picker
  inside it is how a part comes to play it.
- **Transpose is one control**, a switch and a number written together, because that is
  what the panel's button is. The instrument ignores the amount while the switch is
  clear, and never clears the switch once it is set.
- A picker offers only values the library can name. A file holding one it cannot reads
  as *unrecognized value (6)* and keeps it in the list, so changing away from it can be
  undone.
- Reserved bits, unexplained fields and the library ids that name a program's piano and
  sample are not offered. They are all still in **Advanced**.

**Drawbars** are the widget the crate is named after: pull down to draw out, with the
positions in digits underneath (`88 8800 000`). No hex anywhere in the ordinary view.

Sample instruments show their name and the stretch of keyboard each zone covers, with
root key and top note as note names (`C4` is middle C). Only v2 `.nsmp` content can be
changed; nsmp3/nsmp4 is carried verbatim.

### Advanced

Every document ends with an **Advanced** disclosure, and everything the engineering
build put in front of you is in it: the verify badge, the CBIN header numbers, the
bytes that moved since the tab opened (with the checksum rows annotated so they do not
read as a second edit nobody made), the full field registry with each field's stored
spelling and bit placement, the `{:#?}` dump, and — for something read off the
instrument — the slot's own info and dependency list, on request.

### What lives in `nord-cli` instead

Raw browsing by class number and the sweep-capture tool are an engineer's
`nord raw` and `nord --sweep`; they are not in this window.

## Build and run — native

```sh
nix develop            # from the lab root: cargo, rustc, lld
cd crates
cargo run -p drawbar
```

## Build and serve — web

Everything below runs from `crates/`. The `--target wasm32-unknown-unknown`
builds **must** be run from here or below: `crates/.cargo/config.toml` supplies
`--cfg=web_sys_unstable_apis`, without which WebUSB does not exist in `web-sys`,
and Cargo only finds that file by walking up from the working directory.

```sh
cd crates
nix develop /path/to/lab

cargo build -p drawbar --lib --target wasm32-unknown-unknown --release

nix run nixpkgs#wasm-bindgen-cli -- \
  --target web \
  --out-dir drawbar/pkg \
  target/wasm32-unknown-unknown/release/drawbar.wasm

cd drawbar && python3 -m http.server 8000
```

Then open <http://localhost:8000/>. `localhost` counts as a secure context, so
WebUSB is available without TLS; `file://` is **not** — the module import and
WebUSB both fail there.

> [!IMPORTANT]
> `--lib` is not optional. The crate also has a binary target of the same name,
> so building both for wasm writes two different modules to the same
> `target/wasm32-unknown-unknown/*/drawbar.wasm`. Cargo warns about the collision
> and does not define which one survives; when it is the binary's stub `main`,
> `wasm-bindgen` emits a package that exports nothing.

⚠️ **`wasm-bindgen-cli` and the `wasm-bindgen` crate must be the same version.**
The CLI refuses a module built by any other one. nixpkgs currently ships
**0.2.121**, which is what `nord-web-demo` pins and therefore what the whole
workspace lock holds:

```sh
grep -A2 'name = "wasm-bindgen"' Cargo.lock
nix run nixpkgs#wasm-bindgen-cli -- --version
```

If those ever disagree, move the pin in both `nord-web-demo/Cargo.toml` and
`drawbar/Cargo.toml` to whatever the CLI reports and re-run `cargo build`. eframe
resolves against the pin rather than forcing it, so the pin is the one to follow.

## Browser support

| Browser | Works |
|---|---|
| Chrome | yes |
| Edge | yes |
| Firefox | no — WebUSB declined |
| Safari | no — WebUSB declined |

**Close Nord Sound Manager before connecting.** It claims the vendor interface
exclusively, and nothing else — this app, `nord-cli`, or Chrome — can attach
alongside it.

## Disclaimer

Not affiliated with, authorized, or endorsed by Clavia DMI AB. "Nord", "Clavia",
and "Electro" are trademarks of Clavia DMI AB, used here only to identify the
hardware these formats come from.
