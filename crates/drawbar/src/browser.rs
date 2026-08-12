//! The sidebar: the two places a sound can live, and the moving of sounds between them.
//!
//! Nothing here touches the instrument. Rendering reads the caches and answers with a
//! list of [`Act`]s, which [`apply`] then runs against the workspace, the device and the
//! tabs — so a row can be drawn while the thing it stands for is about to change.

use std::sync::Arc;

use eframe::egui;
use nord_format::Entity;
use nord_usb::{Location, ObjectClass};

use crate::app::dot;
use crate::device::{
    holdings, put_refusal, read_only, Connection, Device, DeviceCmd, Outgoing, BROWSED,
};
use crate::log::Log;
use crate::strings::{folder, place, shown};
use crate::tabs::Tabs;
use crate::workspace::{ExportWhat, Fresh, Workspace};

/// What an asset is, which is what decides the folder it belongs in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Program,
    SetList,
    Sample,
    Piano,
    Live,
    Settings,
    /// Something the instrument has no folder for — a bundle, a Stage file, a file that
    /// did not decode at all.
    Other,
}

impl Kind {
    pub fn of(entity: Option<&Entity>) -> Kind {
        match entity {
            Some(Entity::Program(_)) => Kind::Program,
            Some(Entity::Song(_)) => Kind::SetList,
            Some(Entity::Sample(_)) => Kind::Sample,
            Some(Entity::Piano(_) | Entity::PianoLibrary(_)) => Kind::Piano,
            Some(Entity::Live(_)) => Kind::Live,
            Some(Entity::Settings(_)) => Kind::Settings,
            _ => Kind::Other,
        }
    }

    pub fn from_class(class: ObjectClass) -> Kind {
        match class {
            ObjectClass::Program => Kind::Program,
            ObjectClass::SetList => Kind::SetList,
            ObjectClass::Sample => Kind::Sample,
            ObjectClass::Piano => Kind::Piano,
            ObjectClass::Live => Kind::Live,
            ObjectClass::Settings => Kind::Settings,
            ObjectClass::Unknown(_) => Kind::Other,
        }
    }

    /// The folder on the instrument this kind belongs in.
    pub fn home(self) -> Option<ObjectClass> {
        match self {
            Kind::Program => Some(ObjectClass::Program),
            Kind::SetList => Some(ObjectClass::SetList),
            Kind::Sample => Some(ObjectClass::Sample),
            Kind::Piano => Some(ObjectClass::Piano),
            Kind::Live => Some(ObjectClass::Live),
            Kind::Settings => Some(ObjectClass::Settings),
            Kind::Other => None,
        }
    }

    /// The small word next to a row's name.
    pub fn chip(self) -> &'static str {
        match self {
            Kind::Program => "program",
            Kind::SetList => "set list",
            Kind::Sample => "sample",
            Kind::Piano => "piano",
            Kind::Live => "live",
            Kind::Settings => "settings",
            Kind::Other => "file",
        }
    }
}

/// One row of the tree.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Local(u64),
    Slot { class: ObjectClass, at: Location },
}

/// What is under the pointer while a drag is in progress.
#[derive(Clone)]
pub struct Carried {
    pub from: Item,
    pub kind: Kind,
    pub name: String,
}

/// Where a drop would land.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Onto {
    Computer,
    Slot { class: ObjectClass, at: Location },
}

/// What a drop would do, or the plain reason it would do nothing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Landing {
    /// Device to this computer: a copy comes back.
    Copy,
    /// This computer to a slot.
    Send,
    /// Slot to slot inside one folder. The instrument swaps them.
    Rearrange,
    No(&'static str),
}

impl Landing {
    pub fn allowed(self) -> bool {
        !matches!(self, Landing::No(_))
    }
}

/// Whether a drag can end where the pointer is, and what it would mean if it did.
pub fn landing(carried: &Carried, onto: Onto) -> Landing {
    match (carried.from, onto) {
        (Item::Local(_), Onto::Computer) => Landing::No("it is already on this computer"),
        (Item::Local(_), Onto::Slot { class, .. }) => {
            if read_only(class) {
                Landing::No("pianos are installed on the instrument, not moved into it")
            } else if put_refusal(class).is_some() {
                Landing::No("this folder cannot be written to over USB")
            } else if carried.kind.home() != Some(class) {
                Landing::No("that folder holds a different kind of thing")
            } else {
                Landing::Send
            }
        }
        (Item::Slot { .. }, Onto::Computer) => Landing::Copy,
        (
            Item::Slot {
                class: from,
                at: was,
            },
            Onto::Slot { class, at },
        ) => {
            if from != class {
                Landing::No("things only move within their own folder")
            } else if read_only(class) {
                Landing::No("pianos stay where the instrument put them")
            } else if was == at {
                Landing::No("it is already there")
            } else {
                Landing::Rearrange
            }
        }
    }
}

/// What the browser asks the rest of the app to do.
pub enum Act {
    Connect,
    Disconnect,
    OpenFiles,
    New(Fresh),
    ReadAgain(ObjectClass),
    Open(Item),
    Copy {
        class: ObjectClass,
        at: Location,
    },
    LoadOnInstrument {
        class: ObjectClass,
        at: Location,
    },
    /// Put a local asset into a slot, asking first if something is already there.
    Send {
        id: u64,
        class: ObjectClass,
        at: Location,
    },
    /// Write everything waiting, grouped by folder. Already agreed to.
    SendAll,
    /// The same as a Send, already agreed to. Nothing asks twice.
    Replace {
        id: u64,
        class: ObjectClass,
        at: Location,
    },
    Rearrange {
        class: ObjectClass,
        from: Location,
        to: Location,
    },
    RenameLocal {
        id: u64,
        name: String,
    },
    RenameSlot {
        class: ObjectClass,
        at: Location,
        name: String,
    },
    DuplicateLocal(u64),
    DuplicateSlot {
        class: ObjectClass,
        from: Location,
        to: Location,
    },
    DeleteSlot {
        class: ObjectClass,
        at: Location,
    },
    Remove(u64),
    Save(u64),
    /// Nothing happened, and this is why.
    Refused(String),
}

/// An in-place rename, waiting on Enter or Esc.
struct Rename {
    what: Item,
    text: String,
    /// The first frame, in which the field takes focus and selects what is in it.
    fresh: bool,
}

/// A question that has to be answered before something is lost.
struct Ask {
    title: String,
    note: Option<String>,
    verb: &'static str,
    act: Act,
}

/// Whether a plain click starts a rename rather than moving the selection.
///
/// Both halves are needed. Selecting is the whole row's job, so a row that answers a
/// click anywhere would otherwise arm the editor on every second click.
pub fn arms_rename(selected: bool, on_name: bool) -> bool {
    selected && on_name
}

/// What Enter does to an in-place rename: nothing, or a new name.
///
/// A blank field is not a name and an unchanged one is not a rename, so both leave the
/// asset alone rather than sending an operation that would do nothing.
pub fn renamed(original: &str, typed: &str) -> Option<String> {
    let typed = typed.trim();
    match typed.is_empty() || typed == original.trim() {
        true => None,
        false => Some(typed.to_string()),
    }
}

pub struct Browser {
    selection: Option<Item>,
    rename: Option<Rename>,
    ask: Option<Ask>,
    /// Where the divider sits between the two columns, as a share of the dock.
    split: f32,
}

impl Default for Browser {
    fn default() -> Browser {
        Browser {
            selection: None,
            rename: None,
            ask: None,
            split: EVEN,
        }
    }
}

/// The divider's home: half the dock each.
const EVEN: f32 = 0.5;

/// Neither column may be dragged out of existence.
const LEAST: f32 = 0.15;

/// The strip the divider answers on.
const HANDLE: f32 = 7.0;

impl Browser {
    /// Where the divider is kept between sessions.
    pub const SPLIT: &'static str = "drawbar.dock_split";

    /// Put the divider back where it was left.
    ///
    /// Anything the store cannot account for is the even split: a fraction outside the
    /// stops would be one column showing and the other a sliver.
    pub fn restore(&mut self, storage: &dyn eframe::Storage) {
        self.split = storage
            .get_string(Browser::SPLIT)
            .and_then(|text| text.parse::<f32>().ok())
            .filter(|share| (LEAST..=1.0 - LEAST).contains(share))
            .unwrap_or(EVEN);
    }

    pub fn keep(&self, storage: &mut dyn eframe::Storage) {
        storage.set_string(Browser::SPLIT, self.split.to_string());
    }

    /// Draw the places a sound can live and collect what the user asked for.
    ///
    /// The instrument gets a column once there is an instrument. Attached, the two sit
    /// adjacent — a drag between them is then a short horizontal move, and a long device
    /// tree cannot push the local list off-screen. Unattached, the column would be half
    /// the sidebar saying nothing, so the list takes the whole width and the way to an
    /// instrument is a button beside the ones that open files.
    pub fn ui(&mut self, ui: &mut egui::Ui, workspace: &Workspace, device: &Device) -> Vec<Act> {
        let mut acts = Vec::new();
        self.dialog(ui.ctx(), &mut acts);
        match device.state.connected() {
            true => self.dock(ui, workspace, device, &mut acts),
            false => self.computer(ui, workspace, device, &mut acts),
        }
        ghost(ui.ctx());
        acts
    }

    /// The two columns and the divider between them.
    ///
    /// ⚠️ A share of the width rather than a number of points. The sidebar the two live
    /// in is itself resizable, and a column pinned to points eats the other one as the
    /// sidebar narrows — at which point the divider has nothing left to give back.
    fn dock(
        &mut self,
        ui: &mut egui::Ui,
        workspace: &Workspace,
        device: &Device,
        acts: &mut Vec<Act>,
    ) {
        let whole = ui.available_rect_before_wrap();
        let usable = (whole.width() - HANDLE).max(1.0);
        let left = usable * self.split;
        let divider = egui::Rect::from_min_size(
            egui::pos2(whole.left() + left, whole.top()),
            egui::vec2(HANDLE, whole.height()),
        );

        let dragging = ui
            .interact(
                divider,
                ui.id().with("dock_divider"),
                egui::Sense::click_and_drag(),
            )
            .on_hover_and_drag_cursor(egui::CursorIcon::ResizeHorizontal);
        if dragging.dragged() {
            self.split = ((left + dragging.drag_delta().x) / usable).clamp(LEAST, 1.0 - LEAST);
        }
        // Somewhere to put it back to, for a divider that has been dragged into a corner.
        if dragging.double_clicked() {
            self.split = EVEN;
        }

        let ends = |from: f32, to: f32| {
            egui::Rect::from_min_max(
                egui::pos2(from, whole.top()),
                egui::pos2(to, whole.bottom()),
            )
        };
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(ends(whole.left(), divider.left())),
            |ui| self.computer(ui, workspace, device, acts),
        );
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(ends(divider.right(), whole.right())),
            |ui| self.instrument(ui, workspace, device, acts),
        );

        let visuals = ui.visuals();
        let stroke = match dragging.hovered() || dragging.dragged() {
            true => egui::Stroke::new(2.0, visuals.selection.stroke.color),
            false => visuals.widgets.noninteractive.bg_stroke,
        };
        ui.painter()
            .vline(divider.center().x, whole.y_range(), stroke);
        ui.advance_cursor_after_rect(whole);
    }

    /// A column heading, and the strip of buttons under it.
    fn heading(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        buttons: impl FnOnce(&mut egui::Ui),
    ) -> egui::Response {
        let head = ui
            .horizontal(|ui| {
                ui.label(egui::RichText::new(title).strong());
                buttons(ui);
            })
            .response;
        ui.separator();
        head
    }

    fn select(&mut self, item: Item) {
        let same = self.rename.as_ref().is_some_and(|r| r.what == item);
        if !same {
            self.rename = None;
        }
        self.selection = Some(item);
    }

    fn start_rename(&mut self, what: Item, from: &str) {
        self.selection = Some(what);
        self.rename = Some(Rename {
            what,
            text: from.to_string(),
            fresh: true,
        });
    }

    // ---- this computer ----------------------------------------------------------

    fn computer(
        &mut self,
        ui: &mut egui::Ui,
        workspace: &Workspace,
        device: &Device,
        acts: &mut Vec<Act>,
    ) {
        let mut open_files = false;
        let mut fresh = None;
        let mut connect = false;
        let attached = device.state.connected();
        let connecting = matches!(device.state.connection, Connection::Connecting);
        let head = self.heading(ui, "This computer", |ui| {
            open_files = ui.small_button("Open…").clicked();
            ui.menu_button("New", |ui| {
                for kind in Fresh::ALL {
                    if ui.button(kind.label()).clicked() {
                        fresh = Some(kind);
                        ui.close();
                    }
                }
            });
            if attached {
                return;
            }
            match connecting {
                true => {
                    ui.spinner();
                }
                // ⚠️ Reached inside the frame the click landed in, which is what keeps
                // the browser's transient user activation alive for `requestDevice()`.
                false => {
                    connect = ui
                        .small_button("Connect instrument")
                        .on_hover_text(
                            "Close Nord Sound Manager first — it holds the instrument on its \
                             own, and nothing else can reach it alongside.\n\nIn a browser: \
                             Chrome or Edge only.",
                        )
                        .clicked();
                }
            }
        });
        if open_files {
            acts.push(Act::OpenFiles);
        }
        if let Some(kind) = fresh {
            acts.push(Act::New(kind));
        }
        if connect {
            acts.push(Act::Connect);
        }
        // The heading takes a drop, so there is one target that is never also a place a
        // drag could have started from.
        self.drop_zone(ui, &head, Onto::Computer, acts);

        egui::ScrollArea::vertical()
            .id_salt("computer_scroll")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if workspace.entities().is_empty() {
                    ui.label(
                        egui::RichText::new("Drop Nord files here, or use Open…")
                            .weak()
                            .italics(),
                    );
                }
                for entity in workspace.entities() {
                    self.local_row(ui, entity, acts);
                }
                if egui::DragAndDrop::has_payload_of_type::<Carried>(ui.ctx()) {
                    let landing = row(
                        ui,
                        false,
                        &Cells {
                            name: "Drop here to copy it to this computer",
                            faint: true,
                            ..Cells::default()
                        },
                    );
                    self.drop_zone(ui, &landing.response, Onto::Computer, acts);
                }
            });
    }

    fn local_row(
        &mut self,
        ui: &mut egui::Ui,
        entity: &crate::workspace::LocalEntity,
        acts: &mut Vec<Act>,
    ) {
        let item = Item::Local(entity.id);
        let kind = Kind::of(entity.entity.as_ref());
        let selected = self.selection == Some(item);

        // While a name is being typed the row stops sensing anything: a drag sense over
        // the field would take the clicks that place the cursor in it.
        if self.rename.as_ref().is_some_and(|r| r.what == item) {
            if let Some(name) = self.rename_row(ui, &entity.name) {
                acts.push(Act::RenameLocal {
                    id: entity.id,
                    name,
                });
            }
            return;
        }

        let owed = entity.pending.then(|| destination(entity)).flatten();
        let drawn = row(
            ui,
            selected,
            &Cells {
                name: &entity.name,
                note: owed.as_deref().or(Some(kind.chip())),
                dirty: entity.dirty,
                waiting: owed.is_some(),
                ..Cells::default()
            },
        );
        let response = drawn.response;

        if response.dragged() {
            egui::DragAndDrop::set_payload(
                ui.ctx(),
                Carried {
                    from: item,
                    kind,
                    name: entity.name.clone(),
                },
            );
        }
        // A drop onto a row is a drop onto the list; it is taken here so the column's
        // own zone does not act on it a second time.
        self.drop_zone(ui, &response, Onto::Computer, acts);

        if response.double_clicked() {
            acts.push(Act::Open(item));
        } else if response.clicked() {
            self.clicked(item, &response, drawn.name, &entity.name);
        }
        if selected && ui.input(|i| i.key_pressed(egui::Key::F2)) {
            self.start_rename(item, &entity.name);
        }

        response.context_menu(|ui| {
            self.select(item);
            if ui.button("Open").clicked() {
                acts.push(Act::Open(item));
                ui.close();
            }
            if ui.button("Export…").clicked() {
                acts.push(Act::Save(entity.id));
                ui.close();
            }
            if ui.button("Rename").clicked() {
                self.start_rename(item, &entity.name);
                ui.close();
            }
            if ui.button("Duplicate").clicked() {
                acts.push(Act::DuplicateLocal(entity.id));
                ui.close();
            }
            ui.separator();
            if ui.button("Remove from list").clicked() {
                acts.push(Act::Remove(entity.id));
                ui.close();
            }
        });
    }

    /// What a plain click on a row does.
    ///
    /// ⚠️ Arming the rename editor needs the click to land on the **name**, not merely
    /// on a row that was already selected. An editor armed by any second click sits
    /// there with the whole name selected, so the next keystroke — one meant for the
    /// document, or a stray one — replaces it, and the blur commits the replacement.
    /// That is how a program came to be called "0".
    fn clicked(&mut self, item: Item, response: &egui::Response, name: egui::Rect, from: &str) {
        let on_name = response
            .interact_pointer_pos()
            .is_some_and(|at| name.contains(at));
        match arms_rename(self.selection == Some(item), on_name) {
            true => self.start_rename(item, from),
            false => self.select(item),
        }
    }

    // ---- the instrument ---------------------------------------------------------

    fn instrument(
        &mut self,
        ui: &mut egui::Ui,
        workspace: &Workspace,
        device: &Device,
        acts: &mut Vec<Act>,
    ) {
        let Some(product) = device.state.product().map(str::to_string) else {
            return;
        };
        let mut disconnect = false;
        let mut send_all = false;
        let owed = workspace.pending().len();
        let firmware = device.state.firmware();
        self.heading(ui, &product, |ui| {
            dot(ui, crate::app::good(ui.visuals())).on_hover_text("attached");
            if let Some(firmware) = &firmware {
                ui.label(egui::RichText::new(firmware).small().weak())
                    .on_hover_text("the firmware version the instrument reports");
            }
            disconnect = ui.small_button("Disconnect").clicked();
            if owed > 0 {
                send_all = ui
                    .button(egui::RichText::new(format!("Send all ({owed})")).strong())
                    .on_hover_text("write every waiting sound back to the instrument")
                    .clicked();
            }
        });
        if disconnect {
            acts.push(Act::Disconnect);
        }
        if send_all {
            self.ask_send_all(workspace, device);
        }
        egui::ScrollArea::vertical()
            .id_salt("instrument_scroll")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for class in BROWSED {
                    self.class(ui, device, class, acts);
                }
            });
    }

    fn class(
        &mut self,
        ui: &mut egui::Ui,
        device: &Device,
        class: ObjectClass,
        acts: &mut Vec<Act>,
    ) {
        let progress = device.state.scan.progress(class);
        let title = match progress {
            Some(p) if p.running => match p.total {
                Some(total) => format!("{}  ·  reading {} of {total}", folder(class), p.done + 1),
                None => format!("{}  ·  reading…", folder(class)),
            },
            _ => folder(class).to_string(),
        };
        egui::CollapsingHeader::new(title)
            .id_salt(class.to_raw())
            .default_open(matches!(class, ObjectClass::Program))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("Read again").clicked() {
                        acts.push(Act::ReadAgain(class));
                    }
                    if read_only(class) {
                        ui.label(egui::RichText::new("read only").small().weak());
                    }
                    if let Some(holdings) = holdings(class, &device.state.inventory) {
                        ui.label(egui::RichText::new(holdings).small().weak());
                    }
                });
                // One flat list, labelled the way the panel and the CLI label a slot.
                // Banks are a numbering, not a container. A bank the device gave a name
                // of its own gets a caption over its rows — for pianos those names are
                // the panel's categories, and this is the only place they appear.
                let banks = device.state.banks_of(class);
                if banks.is_empty() {
                    ui.label(egui::RichText::new("nothing read yet").small().weak());
                }
                for bank in banks {
                    let Some(slots) = device.state.bank(class, bank) else {
                        continue;
                    };
                    if let Some(name) = device
                        .state
                        .bank_name(class, bank)
                        .filter(|name| worth_captioning(bank, name))
                    {
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(format!("{bank} · {name}"))
                                .small()
                                .weak(),
                        );
                    }
                    for index in 0..slots.len() {
                        let at = Location::from_user(bank, index as u32 + 1);
                        self.slot_row(ui, device, class, at, acts);
                    }
                }
            });
    }

    fn slot_row(
        &mut self,
        ui: &mut egui::Ui,
        device: &Device,
        class: ObjectClass,
        at: Location,
        acts: &mut Vec<Act>,
    ) {
        let held = device
            .state
            .slot(class, at)
            .flatten()
            .map(|info| info.name.trim().to_string());
        let item = Item::Slot { class, at };
        let selected = self.selection == Some(item);

        // While a name is being typed the row stops sensing anything: a drag sense over
        // the field would take the clicks that place the cursor in it.
        if self.rename.as_ref().is_some_and(|r| r.what == item) {
            let was = held.clone().unwrap_or_default();
            if let Some(name) = self.rename_row(ui, &was) {
                acts.push(Act::RenameSlot { class, at, name });
            }
            return;
        }

        let loaded = device.state.focused(class) == Some(at);
        let drawn = row(
            ui,
            selected,
            &Cells {
                at: Some(shown(at)),
                name: held.as_deref().unwrap_or("empty"),
                faint: held.is_none(),
                loaded,
                ..Cells::default()
            },
        );
        let mut response = drawn.response;
        if loaded {
            response = response.on_hover_text("on the instrument's panel now");
        }

        // ⚠️ A piano is a library of hundreds of megabytes, read whole into memory by
        // anything that fetches it. The folder lists what is installed and offers no way
        // to pull one down, which is also what read-only means here.
        let fetchable = !read_only(class);

        if let Some(name) = &held {
            if fetchable && response.dragged() {
                egui::DragAndDrop::set_payload(
                    ui.ctx(),
                    Carried {
                        from: item,
                        kind: Kind::from_class(class),
                        name: name.clone(),
                    },
                );
            }
        }
        self.drop_zone(ui, &response, Onto::Slot { class, at }, acts);

        if response.double_clicked() {
            if held.is_some() && fetchable {
                acts.push(Act::Open(item));
            }
        } else if response.clicked() {
            match (&held, fetchable) {
                (Some(name), true) => self.clicked(item, &response, drawn.name, name),
                _ => self.select(item),
            }
        }
        if let Some(name) = &held {
            if selected && fetchable && ui.input(|i| i.key_pressed(egui::Key::F2)) {
                self.start_rename(item, name);
            }
        }

        let Some(name) = held else {
            return;
        };
        response.context_menu(|ui| {
            self.select(item);
            if !fetchable {
                ui.label(
                    egui::RichText::new("Installed on the instrument; nothing to change here.")
                        .weak(),
                );
                return;
            }
            if ui.button("Open").clicked() {
                acts.push(Act::Open(item));
                ui.close();
            }
            if ui.button("Copy to this computer").clicked() {
                acts.push(Act::Copy { class, at });
                ui.close();
            }
            if ui.button("Load on instrument").clicked() {
                acts.push(Act::LoadOnInstrument { class, at });
                ui.close();
            }
            ui.separator();
            if ui.button("Rename").clicked() {
                self.start_rename(item, &name);
                ui.close();
            }
            let free = device.state.first_free(class);
            if ui
                .add_enabled(free.is_some(), egui::Button::new("Duplicate"))
                .on_disabled_hover_text("every slot read so far is taken")
                .clicked()
            {
                if let Some(to) = free {
                    acts.push(Act::DuplicateSlot {
                        class,
                        from: at,
                        to,
                    });
                }
                ui.close();
            }
            ui.separator();
            if ui.button("Delete…").clicked() {
                self.ask = Some(Ask {
                    title: format!("Delete “{name}” from {}?", place(class, at)),
                    note: Some("It is removed from the instrument. There is no undo.".into()),
                    verb: "Delete",
                    act: Act::DeleteSlot { class, at },
                });
                ui.close();
            }
        });
    }

    // ---- shared pieces ----------------------------------------------------------

    /// The in-place editor, prefilled and selected.
    ///
    /// ⚠️ **Only Enter renames.** Clicking away cancels. An editor that commits on blur
    /// turns a stray keystroke into a rename nobody asked for, and the name is the only
    /// record of what an object is — files store no name of their own.
    fn rename_row(&mut self, ui: &mut egui::Ui, original: &str) -> Option<String> {
        let rename = self.rename.as_mut()?;
        let output = ui
            .horizontal(|ui| {
                egui::TextEdit::singleline(&mut rename.text)
                    .desired_width(ui.available_width())
                    .show(ui)
            })
            .inner;
        if rename.fresh {
            rename.fresh = false;
            output.response.request_focus();
            let all = egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(rename.text.chars().count()),
            );
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), output.response.id) {
                state.cursor.set_char_range(Some(all));
                state.store(ui.ctx(), output.response.id);
            }
            return None;
        }
        // ⚠️ The field's own Enter, not the frame's. `Ui::input` answers for the whole
        // app: an Enter meant for a knob's number box or a table cell reads as one here
        // too, and would close an editor the operator has not finished with. A single
        // line surrenders the focus on Enter, so the two together are the gesture.
        let lost = output.response.lost_focus();
        let entered = lost && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if !lost {
            return None;
        }
        let typed = std::mem::take(&mut rename.text);
        self.rename = None;
        entered.then(|| renamed(original, &typed)).flatten()
    }

    /// Take a drop, if this is somewhere the dragged thing can land.
    ///
    /// A target that would refuse does not light up; dropping on it anyway says why in
    /// the status strip rather than silently doing nothing.
    fn drop_zone(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        onto: Onto,
        acts: &mut Vec<Act>,
    ) {
        if let Some(carried) = response.dnd_hover_payload::<Carried>() {
            if landing(&carried, onto).allowed() {
                ui.painter().rect_stroke(
                    response.rect,
                    3.0,
                    egui::Stroke::new(1.0, ui.visuals().selection.stroke.color),
                    egui::StrokeKind::Inside,
                );
            }
        }
        let Some(carried) = response.dnd_release_payload::<Carried>() else {
            return;
        };
        self.land(&carried, onto, acts);
    }

    fn land(&mut self, carried: &Arc<Carried>, onto: Onto, acts: &mut Vec<Act>) {
        match (landing(carried, onto), carried.from, onto) {
            (Landing::Copy, Item::Slot { class, at }, _) => acts.push(Act::Copy { class, at }),
            (Landing::Rearrange, Item::Slot { at: from, .. }, Onto::Slot { class, at }) => acts
                .push(Act::Rearrange {
                    class,
                    from,
                    to: at,
                }),
            (Landing::Send, Item::Local(id), Onto::Slot { class, at }) => {
                acts.push(Act::Send { id, class, at })
            }
            (Landing::No(why), ..) => acts.push(Act::Refused(format!(
                "“{}” cannot go there — {why}.",
                carried.name
            ))),
            // Every allowed pairing is spelled out above; a shape that reaches here is a
            // verdict about a drag that did not come from where it says it did.
            _ => {}
        }
    }

    /// Ask before a slot is replaced or emptied. The only dialogs left in the app.
    fn dialog(&mut self, ctx: &egui::Context, acts: &mut Vec<Act>) {
        let Some(ask) = &self.ask else {
            return;
        };
        let mut decision = None;
        egui::Modal::new(egui::Id::new("browser_ask")).show(ctx, |ui| {
            ui.set_width(400.0);
            ui.heading(&ask.title);
            if let Some(note) = &ask.note {
                ui.add_space(4.0);
                ui.label(note);
            }
            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    decision = Some(false);
                }
                if ui
                    .add(egui::Button::new(egui::RichText::new(ask.verb).strong()))
                    .clicked()
                {
                    decision = Some(true);
                }
            });
        });
        match decision {
            Some(true) => {
                if let Some(ask) = self.ask.take() {
                    acts.push(ask.act);
                }
            }
            Some(false) => self.ask = None,
            None => {}
        }
    }

    /// The one question a batch asks: everything it is about to write, and what it
    /// would replace.
    fn ask_send_all(&mut self, workspace: &Workspace, device: &Device) {
        let waiting = workspace.pending();
        if waiting.is_empty() {
            return;
        }
        let mut lines = Vec::new();
        for entity in &waiting {
            let Some((class, at)) = entity.origin.slot() else {
                continue;
            };
            let where_ = place(class, at);
            // Naming the occupant is the whole point of asking: a batch is where an
            // operator stops reading each destination for themselves.
            lines.push(match device.state.slot(class, at).flatten() {
                Some(info) => format!(
                    "“{}” replaces “{}” in {where_}",
                    entity.name,
                    info.name.trim()
                ),
                None => format!("“{}” goes into {where_}, which is empty", entity.name),
            });
        }
        self.ask = Some(Ask {
            title: match lines.len() {
                1 => "Send 1 sound to the instrument?".to_string(),
                n => format!("Send {n} sounds to the instrument?"),
            },
            note: Some(lines.join("\n")),
            verb: "Send",
            act: Act::SendAll,
        });
    }

    /// Raise the one Finder-style question a drop can need: the destination is taken.
    fn ask_replace(&mut self, occupant: &str, incoming: &str, at: String, act: Act) {
        self.ask = Some(Ask {
            title: format!("Replace “{occupant}” in {at} with “{incoming}”?"),
            note: Some(format!(
                "“{occupant}” is read back first and put where it was if anything goes \
                 wrong."
            )),
            verb: "Replace",
            act,
        });
    }
}

/// What one row of a list shows.
#[derive(Default)]
pub struct Cells<'a> {
    /// The monospace location column, `7:4`. Assets on this computer have none.
    pub at: Option<String>,
    pub name: &'a str,
    /// A faint word after the name — what kind of thing it is, or where it is owed.
    pub note: Option<&'a str>,
    /// The note is a destination rather than a kind, so it is worth noticing.
    pub waiting: bool,
    /// The name is a stand-in rather than a real one.
    pub faint: bool,
    pub dirty: bool,
    /// The instrument's panel has this slot loaded.
    pub loaded: bool,
}

/// A drawn row: what it answered, and where its name ended up.
pub struct Drawn {
    pub response: egui::Response,
    /// The name's own rectangle — the only sub-area of a row that means anything, and
    /// only because clicking the name of a selected row starts a rename.
    pub name: egui::Rect,
}

/// The width the location column takes, so names line up under each other.
const AT_W: f32 = 42.0;

/// One row of a list: a full-width click target with its text painted into it.
///
/// ⚠️ Nothing inside is a widget. A label allocates a hover rect of its own, which then
/// wins the hit test over the row — the highlight drops out as the pointer crosses the
/// text, and clicks land on whichever word happens to be under them. The row is the only
/// thing that senses.
fn row(ui: &mut egui::Ui, selected: bool, cells: &Cells) -> Drawn {
    let height = ui.text_style_height(&egui::TextStyle::Body) + 4.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click_and_drag(),
    );

    let visuals = ui.visuals();
    let fill = match (selected, response.hovered()) {
        (true, _) => Some(visuals.selection.bg_fill),
        (false, true) => Some(visuals.faint_bg_color),
        (false, false) => None,
    };
    // ⚠️ A selected row is painted in the instrument's red, and the body grey a row is
    // otherwise written in does not survive it. The colour that goes with the fill is the
    // one egui keeps beside it.
    let ink = match selected {
        true => visuals.selection.stroke.color,
        false => visuals.text_color(),
    };
    let weak = match selected {
        true => ink.gamma_multiply(visuals.weak_text_alpha),
        false => visuals.weak_text_color(),
    };
    let strong = match cells.faint {
        true => weak,
        false => ink,
    };
    let painter = ui.painter().clone();
    if let Some(fill) = fill {
        painter.rect_filled(rect, 3.0, fill);
    }

    let mut x = rect.left() + 4.0;
    let gutter = egui::pos2(x + 3.5, rect.center().y);
    // One gutter, two marks that never meet: only a local asset is dirty, and only a slot
    // is loaded on the panel. A ring rather than a second disc, because two dots that
    // differ in nothing but colour read as the same mark.
    if cells.dirty {
        painter.circle_filled(gutter, 3.5, crate::app::warn(ui.visuals()));
    } else if cells.loaded {
        painter.circle_stroke(
            gutter,
            3.0,
            egui::Stroke::new(1.5, crate::app::good(ui.visuals())),
        );
    }
    x += 10.0;

    if let Some(at) = &cells.at {
        let galley = painter.layout_no_wrap(at.clone(), egui::FontId::monospace(11.0), weak);
        painter.galley(
            egui::pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            egui::Color32::PLACEHOLDER,
        );
        x += AT_W;
    }

    let font = egui::FontId::proportional(13.0);
    let galley = painter.layout_no_wrap(cells.name.to_string(), font.clone(), strong);
    let at = egui::pos2(x, rect.center().y - galley.size().y / 2.0);
    let name = egui::Rect::from_min_size(at, galley.size());
    x += galley.size().x + 8.0;
    painter.galley(at, galley, egui::Color32::PLACEHOLDER);

    if let Some(note) = cells.note {
        let galley =
            painter.layout_no_wrap(note.to_string(), egui::FontId::proportional(10.0), weak);
        painter.galley(
            egui::pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            egui::Color32::PLACEHOLDER,
        );
    }
    Drawn { response, name }
}

/// Whether a bank's own name says anything the number beside every row does not.
///
/// Programs come back called "Bank 1", "Bank 2" — a caption repeating the number the
/// location column already carries is a line of furniture. Pianos come back called
/// "Grand" and "Upright", which is the whole reason to show a caption at all.
fn worth_captioning(bank: u32, name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && name != bank.to_string()
        && !name.eq_ignore_ascii_case(&format!("bank {bank}"))
}

/// Where an asset is owed, for the badge that says so.
fn destination(entity: &crate::workspace::LocalEntity) -> Option<String> {
    let (class, at) = entity.origin.slot()?;
    Some(format!("will be sent to {}", place(class, at)))
}

/// The name of whatever is being dragged, following the pointer.
fn ghost(ctx: &egui::Context) {
    let Some(carried) = egui::DragAndDrop::payload::<Carried>(ctx) else {
        return;
    };
    let Some(at) = ctx.pointer_interact_pos() else {
        return;
    };
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("drag_ghost"),
    ));
    let where_ = at + egui::vec2(12.0, 6.0);
    let text = painter.layout_no_wrap(
        carried.name.clone(),
        egui::FontId::proportional(12.0),
        ctx.style().visuals.strong_text_color(),
    );
    painter.rect_filled(
        egui::Rect::from_min_size(where_, text.size()).expand(4.0),
        3.0,
        ctx.style().visuals.window_fill,
    );
    painter.galley(where_, text, egui::Color32::PLACEHOLDER);
}

/// Run what the browser asked for.
pub fn apply(
    browser: &mut Browser,
    acts: Vec<Act>,
    workspace: &mut Workspace,
    device: &mut Device,
    tabs: &mut Tabs,
    log: &mut Log,
) {
    for act in acts {
        match act {
            Act::Connect => device.connect(log),
            Act::Disconnect => device.disconnect(log),
            Act::OpenFiles => workspace.open_dialog(),
            Act::New(kind) => {
                if let Some(id) = workspace.create(kind, log) {
                    tabs.open(id, workspace);
                }
            }
            Act::ReadAgain(class) => device.read_class(class),
            Act::Open(Item::Local(id)) => tabs.open(id, workspace),
            Act::Open(Item::Slot { class, at }) => device.send(
                DeviceCmd::Get {
                    class,
                    at,
                    body: false,
                    open: true,
                },
                log,
            ),
            Act::Copy { class, at } => device.send(
                DeviceCmd::Get {
                    class,
                    at,
                    body: false,
                    open: false,
                },
                log,
            ),
            Act::LoadOnInstrument { class, at } => {
                device.send(DeviceCmd::Select { class, at }, log)
            }
            Act::Send { id, class, at } => {
                send(browser, workspace, device, log, id, class, at, true)
            }
            Act::Replace { id, class, at } => {
                send(browser, workspace, device, log, id, class, at, false)
            }
            Act::SendAll => send_all(workspace, device, log),
            Act::Rearrange { class, from, to } => {
                device.send(DeviceCmd::Move { class, from, to }, log)
            }
            Act::RenameLocal { id, name } => {
                workspace.rename(id, name.clone());
                log.say(format!("Renamed it “{name}”."));
            }
            Act::RenameSlot { class, at, name } => {
                device.send(DeviceCmd::Rename { class, at, name }, log)
            }
            Act::DuplicateLocal(id) => {
                workspace.duplicate(id, log);
            }
            Act::DuplicateSlot { class, from, to } => {
                device.send(DeviceCmd::Duplicate { class, from, to }, log)
            }
            Act::DeleteSlot { class, at } => device.send(DeviceCmd::Delete { class, at }, log),
            Act::Remove(id) => {
                tabs.close(id);
                workspace.remove(id, log);
            }
            Act::Save(id) => workspace.export(id, ExportWhat::File),
            Act::Refused(why) => log.say(why),
        }
    }
}

/// Everything waiting, one command per folder.
///
/// Grouped by class because a session belongs to a class: one folder is one session, and
/// the worker keeps it open across every item in it.
fn send_all(workspace: &Workspace, device: &mut Device, log: &mut Log) {
    // Refused before the transport is touched: bytes that are not what they claim to be
    // must not reach a delete-then-write, and one bad item stops the whole batch rather
    // than being skipped past.
    for entity in workspace.pending() {
        if let Err(e) = nord_usb::envelope::unwrap(&entity.bytes) {
            log.error(format!("{}: {e}", entity.name));
            log.trouble(format!(
                "“{}” is not a file the instrument takes, so nothing was sent.",
                entity.name
            ));
            return;
        }
    }
    for (class, items) in grouped(workspace) {
        device.send(DeviceCmd::SendAll { class, items }, log);
    }
}

/// Everything waiting, gathered per folder in the order it was opened.
///
/// A session belongs to a folder, so a folder is the unit a batch is cut into.
fn grouped(workspace: &Workspace) -> Vec<(ObjectClass, Vec<Outgoing>)> {
    let mut by_class: Vec<(ObjectClass, Vec<Outgoing>)> = Vec::new();
    for entity in workspace.pending() {
        let Some((class, at)) = entity.origin.slot() else {
            continue;
        };
        if !crate::device::sendable(class) {
            continue;
        }
        let item = Outgoing {
            id: entity.id,
            at,
            name: entity.name.clone(),
            bytes: entity.bytes.clone(),
        };
        match by_class.iter_mut().find(|(held, _)| *held == class) {
            Some((_, items)) => items.push(item),
            None => by_class.push((class, vec![item])),
        }
    }
    by_class
}

/// Put a local asset into a slot.
///
/// `ask` is false once the replace question has been answered, which is what keeps the
/// answer from raising the question again.
#[allow(clippy::too_many_arguments)]
fn send(
    browser: &mut Browser,
    workspace: &Workspace,
    device: &mut Device,
    log: &mut Log,
    id: u64,
    class: ObjectClass,
    at: Location,
    ask: bool,
) {
    let Some(entity) = workspace.get(id) else {
        return;
    };
    // Refused before the transport is touched: bytes that are not what they claim to be
    // must not reach a delete-then-write.
    if let Err(e) = nord_usb::envelope::unwrap(&entity.bytes) {
        log.error(format!("{}: {e}", entity.name));
        log.trouble(format!(
            "“{}” is not a file the instrument takes.",
            entity.name
        ));
        return;
    }
    let occupant = device
        .state
        .slot(class, at)
        .flatten()
        .map(|info| info.name.trim().to_string());
    match (ask, occupant) {
        (true, Some(occupant)) => browser.ask_replace(
            &occupant,
            &entity.name,
            place(class, at),
            Act::Replace { id, class, at },
        ),
        _ => device.send(
            DeviceCmd::Put {
                id,
                class,
                at,
                name: entity.name.clone(),
                bytes: entity.bytes.clone(),
            },
            log,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(kind: Kind) -> Carried {
        Carried {
            from: Item::Local(1),
            kind,
            name: "Africa Split".into(),
        }
    }

    fn slot(class: ObjectClass, bank: u32, slot: u32) -> Carried {
        Carried {
            from: Item::Slot {
                class,
                at: Location { bank, slot },
            },
            kind: Kind::from_class(class),
            name: "Squabble B".into(),
        }
    }

    fn onto(class: ObjectClass, bank: u32, at: u32) -> Onto {
        Onto::Slot {
            class,
            at: Location { bank, slot: at },
        }
    }

    /// The two crossings the browser exists for.
    #[test]
    fn a_drag_between_the_two_places_copies_one_way_and_sends_the_other() {
        assert_eq!(
            landing(&slot(ObjectClass::Program, 6, 3), Onto::Computer),
            Landing::Copy
        );
        assert_eq!(
            landing(&local(Kind::Program), onto(ObjectClass::Program, 6, 3)),
            Landing::Send
        );
    }

    /// An empty slot is a target like any other — that is the whole reason it is a row.
    #[test]
    fn an_empty_slot_is_a_target() {
        assert_eq!(
            landing(&local(Kind::SetList), onto(ObjectClass::SetList, 0, 12)),
            Landing::Send
        );
    }

    /// A folder holds one kind of thing, and the instrument is not asked to sort it out.
    #[test]
    fn a_thing_cannot_be_dropped_into_a_folder_for_another_kind() {
        for kind in [Kind::SetList, Kind::Sample, Kind::Other] {
            assert!(!landing(&local(kind), onto(ObjectClass::Program, 0, 0)).allowed());
        }
    }

    /// Pianos are listed and never written; live and settings have no proven write path.
    #[test]
    fn the_folders_that_cannot_be_written_refuse_a_drop() {
        for class in [ObjectClass::Piano, ObjectClass::Live, ObjectClass::Settings] {
            let kind = Kind::from_class(class);
            assert!(
                !landing(&local(kind), onto(class, 0, 0)).allowed(),
                "{}",
                folder(class)
            );
        }
    }

    /// Slot to slot is the instrument's swap, and only inside one folder.
    #[test]
    fn slots_rearrange_only_within_their_own_folder() {
        assert_eq!(
            landing(
                &slot(ObjectClass::Program, 6, 3),
                onto(ObjectClass::Program, 7, 12)
            ),
            Landing::Rearrange
        );
        assert!(!landing(
            &slot(ObjectClass::Program, 6, 3),
            onto(ObjectClass::SetList, 0, 0)
        )
        .allowed());
    }

    /// Dropping something back where it came from is not a move.
    #[test]
    fn dropping_a_slot_on_itself_does_nothing() {
        assert!(!landing(
            &slot(ObjectClass::Program, 6, 3),
            onto(ObjectClass::Program, 6, 3)
        )
        .allowed());
        assert!(!landing(&local(Kind::Program), Onto::Computer).allowed());
    }

    /// A refusal carries the words the status strip will show, so there is always
    /// something to say.
    #[test]
    fn every_refusal_explains_itself() {
        let cases = [
            landing(&local(Kind::Program), Onto::Computer),
            landing(&local(Kind::Live), onto(ObjectClass::Live, 0, 0)),
            landing(&local(Kind::Other), onto(ObjectClass::Program, 0, 0)),
            landing(
                &slot(ObjectClass::Program, 0, 0),
                onto(ObjectClass::Sample, 0, 0),
            ),
        ];
        for case in cases {
            match case {
                Landing::No(why) => assert!(!why.is_empty()),
                other => panic!("{other:?} should have been refused"),
            }
        }
    }

    /// Paint the two columns headlessly. What this catches is a layout that panics or
    /// an id that collides, neither of which a unit test on the rules would see.
    fn paint(with_device: bool) {
        use crate::workspace::Fresh;

        let ctx = egui::Context::default();
        let mut workspace = Workspace::new(ctx.clone());
        let mut device = Device::new(ctx.clone());
        let mut log = crate::log::Log::default();
        let mut tabs = Tabs::default();
        let mut browser = Browser::default();

        for kind in [Fresh::Program, Fresh::Live, Fresh::Settings] {
            workspace.create(kind, &mut log).unwrap();
        }
        if with_device {
            // Every row shape a list can hold: a named slot, a vacant one, the slot the
            // panel is on, and a class that was never read.
            device.pretend_scanned(ObjectClass::Program, 7, &["Africa Split", "", "Squabble B"]);
            device.pretend_scanned(ObjectClass::Program, 8, &["Bass Manual"]);
            device.pretend_scanned(ObjectClass::SetList, 1, &["Sunday"]);
            device.pretend_focused(ObjectClass::Program, Location { bank: 6, slot: 2 });
            // Named banks, which is what a piano's categories arrive as.
            device.pretend_scanned(ObjectClass::Piano, 1, &["Royal Grand 3D"]);
            device.pretend_geometry(ObjectClass::Piano, &[("Grand", 1), ("Upright", 1)]);
        }

        // Twice: the second pass runs with the widget state the first left behind.
        for _ in 0..2 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::SidePanel::left("places").show(ctx, |ui| {
                    let acts = browser.ui(ui, &workspace, &device);
                    apply(
                        &mut browser,
                        acts,
                        &mut workspace,
                        &mut device,
                        &mut tabs,
                        &mut log,
                    );
                });
            });
        }
    }

    /// A batch is one command per folder, because a session belongs to a folder — and
    /// something that cannot be written is not queued at all.
    #[test]
    fn a_batch_is_grouped_into_one_command_per_folder() {
        use crate::workspace::{Fresh, Origin};

        let ctx = egui::Context::default();
        let mut workspace = Workspace::new(ctx.clone());
        let mut log = crate::log::Log::default();
        let bytes = {
            let id = workspace.create(Fresh::Program, &mut log).unwrap();
            let bytes = workspace.get(id).unwrap().bytes.clone();
            workspace.remove(id, &mut log);
            bytes
        };
        let at = |slot| Location { bank: 6, slot };
        for (class, slot) in [
            (ObjectClass::Program, 0),
            (ObjectClass::Program, 1),
            (ObjectClass::SetList, 0),
            // Live refuses a write, so it must not reach the queue.
            (ObjectClass::Live, 0),
        ] {
            let id = workspace.ingest(
                format!("{}.ne5p", place(class, at(slot))),
                Origin::Device {
                    class,
                    at: at(slot),
                },
                bytes.clone(),
                &mut log,
            );
            workspace.mark_pending(id, true);
        }

        let queued = grouped(&workspace);
        assert_eq!(queued.len(), 2, "one command per folder");
        let programs = queued
            .iter()
            .find(|(class, _)| *class == ObjectClass::Program)
            .expect("programs are queued");
        assert_eq!(programs.1.len(), 2);
        assert!(queued.iter().all(|(class, _)| *class != ObjectClass::Live));
    }

    /// A lone send names the asset it is sending, so the write can pay off that one
    /// document's debt the way a batch pays off its own.
    #[test]
    fn sending_one_document_names_the_asset_it_sends() {
        use crate::workspace::{Fresh, Origin};

        let ctx = egui::Context::default();
        let mut workspace = Workspace::new(ctx.clone());
        let mut device = Device::new(ctx);
        let mut log = crate::log::Log::default();
        let mut tabs = Tabs::default();
        let mut browser = Browser::default();
        device.pretend_scanned(ObjectClass::Program, 7, &["Africa Split"]);

        let at = Location { bank: 6, slot: 1 };
        let bytes = {
            let id = workspace.create(Fresh::Program, &mut log).unwrap();
            let bytes = workspace.get(id).unwrap().bytes.clone();
            workspace.remove(id, &mut log);
            bytes
        };
        let id = workspace.ingest(
            "Africa-Split.ne5p".into(),
            Origin::Device {
                class: ObjectClass::Program,
                at,
            },
            bytes,
            &mut log,
        );
        workspace.mark_pending(id, true);

        // The slot is empty in the scan, so nothing is asked and the put goes straight out.
        apply(
            &mut browser,
            vec![Act::Send {
                id,
                class: ObjectClass::Program,
                at,
            }],
            &mut workspace,
            &mut device,
            &mut tabs,
            &mut log,
        );
        let queued = device.queued().front().expect("a put was queued");
        match queued {
            DeviceCmd::Put { id: sending, .. } => assert_eq!(*sending, id),
            other => panic!("{}", other.label()),
        }
    }

    /// The divider does what dragging it says, and stops before either column is gone.
    ///
    /// ⚠️ The share is what is kept, not a width: the sidebar the two live in is itself
    /// resizable, so the same fraction has to survive the dock changing size under it.
    #[test]
    fn the_divider_moves_and_stops_short_of_squeezing_a_column_out() {
        use crate::workspace::Fresh;

        let ctx = egui::Context::default();
        let mut workspace = Workspace::new(ctx.clone());
        let mut device = Device::new(ctx.clone());
        let mut log = crate::log::Log::default();
        let mut browser = Browser::default();
        workspace.create(Fresh::Program, &mut log).unwrap();
        device.pretend_scanned(ObjectClass::Program, 7, &["Africa Split"]);

        // Far enough left to ask for more than the stop allows.
        let travel = -400.0;
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };

        let mut divider = egui::pos2(0.0, 0.0);
        let mut frame = 0;
        while frame < 5 {
            let grip = divider;
            let moved = grip + egui::vec2(travel, 0.0);
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 720.0),
                )),
                events: match frame {
                    0 => Vec::new(),
                    1 => vec![egui::Event::PointerMoved(grip)],
                    2 => vec![button(grip, true)],
                    3 => vec![egui::Event::PointerMoved(moved)],
                    _ => vec![button(moved, false)],
                },
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                egui::SidePanel::left("places")
                    .exact_width(600.0)
                    .show(ctx, |ui| {
                        // The same rect the dock lays itself out in, so the test grabs
                        // the divider where the divider actually is.
                        let whole = ui.available_rect_before_wrap();
                        divider = egui::pos2(
                            whole.left() + (whole.width() - HANDLE) * browser.split + HANDLE / 2.0,
                            whole.center().y,
                        );
                        let _ = browser.ui(ui, &workspace, &device);
                    });
            });
            frame += 1;
        }

        assert!(browser.split < EVEN, "it moved: {}", browser.split);
        assert_eq!(browser.split, LEAST, "and stopped at the stop");
    }

    /// A share the store cannot account for is the even split, never one column and a
    /// sliver of the other.
    #[test]
    fn a_divider_comes_back_where_it_was_left_or_not_at_all() {
        struct Fake(Option<String>);
        impl eframe::Storage for Fake {
            fn get_string(&self, _key: &str) -> Option<String> {
                self.0.clone()
            }
            fn set_string(&mut self, _key: &str, value: String) {
                self.0 = Some(value);
            }
            fn flush(&mut self) {}
        }

        let restored = |held: Option<&str>| {
            let mut browser = Browser::default();
            browser.restore(&Fake(held.map(str::to_string)));
            browser.split
        };
        assert_eq!(restored(Some("0.3")), 0.3);
        assert_eq!(restored(None), EVEN);
        for nonsense in ["0.0", "1.0", "-3", "wide", "", "NaN"] {
            assert_eq!(restored(Some(nonsense)), EVEN, "{nonsense:?}");
        }

        // And what is written comes back as itself.
        let mut store = Fake(None);
        let browser = Browser {
            split: 0.42,
            ..Browser::default()
        };
        browser.keep(&mut store);
        let mut after = Browser::default();
        after.restore(&store);
        assert_eq!(after.split, 0.42);
    }

    #[test]
    fn the_two_columns_paint_with_nothing_attached() {
        paint(false);
    }

    #[test]
    fn the_two_columns_paint_with_a_tree_to_show() {
        paint(true);
    }

    /// ⚠️ The gesture that lost a program its name. An editor armed by any second click
    /// on a selected row sits there with everything selected, so the next keystroke
    /// replaces the name and the blur commits it.
    #[test]
    fn a_click_away_from_the_name_selects_rather_than_arming_a_rename() {
        // The row is the click target, so most of it must be safe to click.
        assert!(!arms_rename(true, false), "past the name on a selected row");
        assert!(
            !arms_rename(false, true),
            "on the name of an unselected row"
        );
        assert!(!arms_rename(false, false));
        assert!(arms_rename(true, true), "the one gesture that renames");
    }

    /// The gesture end to end: arm the editor, type, press Enter, and the new name comes
    /// back as an act.
    ///
    /// What this catches is a rename that has stopped committing at all — the unit tests
    /// on [`renamed`] cannot see the field it is fed from, and the field's own idea of
    /// "the operator pressed Enter" is the part that is easy to get wrong.
    #[test]
    fn typing_a_name_and_pressing_enter_renames_the_row() {
        use crate::workspace::Fresh;

        let ctx = egui::Context::default();
        let mut workspace = Workspace::new(ctx.clone());
        let device = Device::new(ctx.clone());
        let mut log = crate::log::Log::default();
        let mut browser = Browser::default();
        let id = workspace.create(Fresh::Program, &mut log).unwrap();

        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        };
        // The editor opens on the first frame and takes the focus; the second types over
        // what it opened with, selected; the third commits.
        let frames: [Vec<egui::Event>; 3] = [
            Vec::new(),
            vec![egui::Event::Text("LA Grand".into())],
            vec![key(egui::Key::Enter)],
        ];
        browser.start_rename(Item::Local(id), "Africa Split");

        let mut named = None;
        for events in frames {
            let input = egui::RawInput {
                events,
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                egui::SidePanel::left("places").show(ctx, |ui| {
                    for act in browser.ui(ui, &workspace, &device) {
                        if let Act::RenameLocal { name, .. } = act {
                            named = Some(name);
                        }
                    }
                });
            });
        }
        assert_eq!(named.as_deref(), Some("LA Grand"));
        assert!(browser.rename.is_none(), "and the editor is done with");
    }

    /// Only Enter renames: an armed editor that commits on blur turns a stray keystroke
    /// into a rename nobody asked for.
    #[test]
    fn a_rename_needs_enter_and_a_real_change() {
        assert_eq!(renamed("Africa Split", "LA Grand"), Some("LA Grand".into()));
        // What blur hands back is nothing at all — see `rename_row`.
        assert_eq!(renamed("Africa Split", "Africa Split"), None);
    }

    /// Enter on an untouched field, or on an empty one, leaves the asset alone.
    #[test]
    fn a_rename_that_changes_nothing_is_not_a_rename() {
        assert_eq!(renamed("Africa Split", "Africa Split"), None);
        assert_eq!(renamed("Africa Split", "  Africa Split "), None);
        assert_eq!(renamed("Africa Split", "   "), None);
        assert_eq!(renamed("Africa Split", ""), None);
    }

    /// What is typed is what the asset is called, with the spaces around it dropped.
    #[test]
    fn a_rename_takes_the_typed_name_trimmed() {
        assert_eq!(
            renamed("Africa Split", "  LA Grand  "),
            Some("LA Grand".into())
        );
    }

    /// A caption earns its line by saying something the location column does not. The
    /// piano categories do; "Bank 1" over the rows already labelled `1:…` does not.
    #[test]
    fn a_bank_caption_only_shows_what_the_number_does_not_say() {
        assert!(worth_captioning(1, "Grand"));
        assert!(worth_captioning(2, "Upright"));
        for furniture in ["Bank 1", "bank 1", "BANK 1", "1", " ", ""] {
            assert!(!worth_captioning(1, furniture), "{furniture:?}");
        }
        // The number has to match to be redundant — "Bank 2" over bank 1 is worth saying,
        // because one of the two is wrong and hiding it would hide that.
        assert!(worth_captioning(1, "Bank 2"));
    }

    /// A folder holds exactly the kind named after it.
    #[test]
    fn every_kind_knows_the_folder_it_belongs_in() {
        for class in BROWSED {
            assert_eq!(Kind::from_class(class).home(), Some(class), "{class:?}");
        }
        assert_eq!(Kind::Other.home(), None);
    }
}
