# drawbar

An [egui](https://github.com/emilk/egui) app over
[`nord-format`](../nord-format) and [`nord-usb`](../nord-usb) — everything
[`nord-cli`](../nord-cli) can do, reachable from a browser tab or a desktop
window. Same job as the CLI: **dogfood the libraries**, not ship a product.

It is named for the nine-drawbar organ register widget the editor will grow.

> [!CAUTION]
> # ⚠️ 🧱 USE AT YOUR OWN RISK — THIS TOOL CAN BRICK YOUR DEVICE 🧱 ⚠️
>
> Until this project reaches 1.0, **do not use this tool to speak to a real Nord
> device** unless you are using it to actively develop these libraries.
>
> **I am not responsible for what you do with this tool.**

## What this build does

The window is four regions:

| region | what it holds |
|---|---|
| **Workspace** (left) | local entities: dropped or opened files, objects pulled off the instrument, fresh defaults, duplicates. Each row carries its format tag, where it came from, and a verify badge |
| **Instrument** (right) | the attached Nord: inventory, a slot browser per object class, and the transfer and organisation verbs |
| **Inspector** (centre) | the selected entity: identity, CBIN header, the field table, format extras, and a raw `{:#?}` dump |
| **Activity** (bottom) | every operation, warning and error, collapsible |

### Files

Files arrive by drag-and-drop or the **Open…** button, and leave either as-is
(**Export file**) or with the CBIN header stripped (**Export raw body**, the
same bytes `nord … get --body` writes). Every entity is decoded and immediately
re-encoded; the badge says whether the bytes came back identical, and names the
offset of the first difference when they did not. A file that fails to decode
still gets a row — reporting a bad file is the point of opening it.

The field table comes from `nord-format`'s generated registry, so it lists what
the library currently knows rather than a handwritten summary that can go stale.

### The instrument

**Connect** opens the device (a chooser in the browser, the first attached
Clavia natively) and reads the inventory. Class tabs cover pianos, samples,
programs, set lists, live and settings, plus **other…** for a class addressed by
number. Inventory gives counts but not names, so a bank's slot names are filled
in one `INFO` at a time by **Scan bank** and cached until something changes them.
Empty slots are rows, not absences: they are valid targets for a put or a move.

Select a slot for the verbs — Get (into the workspace), Get raw body, Info, Deps,
Select, Rename, Move, Duplicate, Delete, and Put from the selected workspace
entity. The **sweep** tool reads the same slot once per change you make on the
panel, filing each capture in the workspace under the label you type.

Progress is painted on the *instrument's own display* by the operations
themselves — there is no host-side progress callback, so the app shows a spinner
and the operation's name and does not invent one.

### Safety

- **Read-only until you say otherwise.** There is no armed mode: a destructive
  session exists only for the single operation a confirmation released.
- **Every mutation names its victim first.** "Overwrite 7:4 'Squabble B' with
  'Africa Split.ne5p'?" Move and duplicate name the *destination's* occupant,
  because that is the thing about to be lost. A destination whose bank has not
  been scanned has no name to quote, so those actions stay disabled until it has.
- **Move is a swap, not an overwrite** — the destination's occupant ends up in
  the source slot, byte-identical. Confirmed on hardware. The confirmation says
  so, because calling it an overwrite would invite deleting the destination first
  and destroy the very thing the swap preserves.
- **A put is a delete followed by a write** — the instrument refuses to overwrite
  in place. The occupant is read into memory first and written back if the write
  fails; if the restore fails too, its bytes land in the workspace as a rescued
  entity rather than being lost.
- **Live and settings refuse a write.** Whether either survives a delete of its
  own class is unconfirmed on hardware, so an edit of either stops at a file.
- **Every session closes, including on the error path.** An abandoned transaction
  strands the instrument on its progress screen with no way out but a power
  cycle.

### Editing

The centre switches between **Inspect** and **Edit**. The editor is the field
list: every field the library declares, with its current value, the values it
accepts, and where its bits sit. Widgets follow the field — a checkbox for a
bool, a menu for a named set, a slider for a gapless numeric range, the stored
bits for anything too wide to enumerate, and **nine drawbars** for the organ
register blocks the crate is named after.

Edits are **staged**. Nothing touches the entity until Apply: the pending
`path = value` sets are replayed onto a fresh decode of the unedited bytes, and
the Changes panel shows each field before→after plus the exact bytes that moved,
with the CBIN checksum rows annotated so they do not read as a second edit
nobody made. Revert drops the list. A value the field cannot hold is refused by
the library, and its message names what the field does accept.

> [!WARNING]
> **Some fields only mean something in pairs.** `center_panel.transpose` is
> ignored while `center_panel.transpose_enabled` is clear, the instrument never
> clears that bit once it is set, and an untouched program holds `+1` rather
> than `0`. Setting one half without the other warns; it is not refused.

In **b3+bass**, preset 1 is the bass manual: only two drawbars are live and they
live outside the nine-nibble block, which holds stale leftovers. The editor says
so and greys those nine bars out rather than offering registration that plays
nothing.

Sample instruments get their own editor — the name, and each zone's root key and
top note as note names (`C4` is middle C), zones numbered from 1 at the top of
the keyboard. Only v2 `.nsmp` content is editable; nsmp3/nsmp4 is carried
verbatim.

Editing a slot on the instrument is **Get → edit → Put**: the read lands in the
workspace, the editor works on that copy, and the Instrument pane writes it back
through the put safety flow. Live and settings stop at a file, as above.

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
