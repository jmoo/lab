//! The sidebar: the two places a sound can live, and the moving of sounds between them.
//!
//! Nothing here touches the instrument. Rendering reads the caches and answers with a
//! list of [`Act`]s, which [`apply`] then runs against the workspace, the device and the
//! tabs — so a row can be drawn while the thing it stands for is about to change.

use std::sync::Arc;

use eframe::egui;
use nord_format::Entity;
use nord_usb::{Location, ObjectClass};

use crate::app::{dot, GOOD, WARN};
use crate::device::{holdings, put_refusal, read_only, Device, DeviceCmd, BROWSED};
use crate::log::Log;
use crate::strings::{folder, place};
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
    /// The same, already agreed to. Nothing asks twice.
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

#[derive(Default)]
pub struct Browser {
    selection: Option<Item>,
    rename: Option<Rename>,
    ask: Option<Ask>,
}

impl Browser {
    /// Draw the sidebar and collect what the user asked for.
    pub fn ui(&mut self, ui: &mut egui::Ui, workspace: &Workspace, device: &Device) -> Vec<Act> {
        let mut acts = Vec::new();
        self.dialog(ui.ctx(), &mut acts);
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                self.computer(ui, workspace, &mut acts);
                ui.add_space(10.0);
                self.instrument(ui, device, &mut acts);
            });
        ghost(ui.ctx());
        acts
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

    fn computer(&mut self, ui: &mut egui::Ui, workspace: &Workspace, acts: &mut Vec<Act>) {
        let head = egui::CollapsingHeader::new(egui::RichText::new("This computer").strong())
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("Open…").clicked() {
                        acts.push(Act::OpenFiles);
                    }
                    ui.menu_button("New", |ui| {
                        for kind in Fresh::ALL {
                            if ui.button(kind.label()).clicked() {
                                acts.push(Act::New(kind));
                                ui.close();
                            }
                        }
                    });
                });
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
                // Somewhere unambiguous to aim at, since every row above is also a
                // place a drag could have started from.
                if egui::DragAndDrop::has_payload_of_type::<Carried>(ui.ctx()) {
                    let landing = row(ui, false, |ui| {
                        ui.label(
                            egui::RichText::new("Drop here to copy it to this computer")
                                .weak()
                                .italics(),
                        );
                    });
                    self.drop_zone(ui, &landing, Onto::Computer, acts);
                }
            });

        // The heading takes a drop too, so the section is a target even when it is
        // collapsed.
        self.drop_zone(ui, &head.header_response, Onto::Computer, acts);
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

        let response = row(ui, selected, |ui| {
            if entity.dirty {
                dot(ui, WARN).on_hover_text("changed since it was opened");
            }
            ui.label(&entity.name);
            ui.label(egui::RichText::new(kind.chip()).small().weak());
        });

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
        // A drop onto a row is a drop onto the list; it is taken here so the section's
        // own zone does not act on it a second time.
        self.drop_zone(ui, &response, Onto::Computer, acts);

        if response.double_clicked() {
            acts.push(Act::Open(item));
        } else if response.clicked() {
            // Clicking the name of an already-selected row is the rename gesture.
            match selected {
                true => self.start_rename(item, &entity.name),
                false => self.select(item),
            }
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
            if ui.button("Save to disk…").clicked() {
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

    // ---- the instrument ---------------------------------------------------------

    fn instrument(&mut self, ui: &mut egui::Ui, device: &Device, acts: &mut Vec<Act>) {
        let Some(product) = device.state.product() else {
            return self.no_instrument(ui, device, acts);
        };
        egui::CollapsingHeader::new(egui::RichText::new(product).strong())
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    dot(ui, GOOD).on_hover_text("attached");
                    if ui.small_button("Disconnect").clicked() {
                        acts.push(Act::Disconnect);
                    }
                });
                for class in BROWSED {
                    self.class(ui, device, class, acts);
                }
            });
    }

    fn no_instrument(&mut self, ui: &mut egui::Ui, device: &Device, acts: &mut Vec<Act>) {
        let connecting = matches!(
            device.state.connection,
            crate::device::Connection::Connecting
        );
        egui::CollapsingHeader::new(egui::RichText::new("No instrument").strong())
            .default_open(true)
            .show(ui, |ui| {
                if connecting {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Waiting for the instrument…");
                    });
                } else if ui.button("Connect").clicked() {
                    // ⚠️ Reached inside the frame the click landed in, which is what
                    // keeps the browser's transient user activation alive for
                    // `requestDevice()`.
                    acts.push(Act::Connect);
                }
                ui.label(
                    egui::RichText::new(
                        "Close Nord Sound Manager first — it holds the instrument on its \
                         own, and nothing else can reach it alongside.",
                    )
                    .small()
                    .weak(),
                );
                ui.label(
                    egui::RichText::new("In a browser: Chrome or Edge only.")
                        .small()
                        .weak(),
                );
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
                let banks = device.state.banks_of(class);
                if banks.is_empty() {
                    ui.label(egui::RichText::new("nothing read yet").small().weak());
                }
                let single = banks.len() == 1;
                for bank in banks {
                    match single {
                        true => self.slots(ui, device, class, bank, acts),
                        false => {
                            egui::CollapsingHeader::new(format!("Bank {bank}"))
                                .id_salt((class.to_raw(), bank))
                                .default_open(bank == 1)
                                .show(ui, |ui| self.slots(ui, device, class, bank, acts));
                        }
                    }
                }
            });
    }

    fn slots(
        &mut self,
        ui: &mut egui::Ui,
        device: &Device,
        class: ObjectClass,
        bank: u32,
        acts: &mut Vec<Act>,
    ) {
        let Some(slots) = device.state.bank(class, bank) else {
            return;
        };
        for index in 0..slots.len() {
            let at = Location::from_user(bank, index as u32 + 1);
            self.slot_row(ui, device, class, at, acts);
        }
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

        let response = row(ui, selected, |ui| {
            ui.label(
                egui::RichText::new(format!("{:>2}", at.slot + 1))
                    .monospace()
                    .small()
                    .weak(),
            );
            match &held {
                Some(name) => {
                    ui.label(name);
                }
                None => {
                    ui.label(egui::RichText::new("empty").weak().italics());
                }
            }
        });

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
            match (selected, &held) {
                (true, Some(name)) if !read_only(class) => self.start_rename(item, name),
                _ => self.select(item),
            }
        }
        if let Some(name) = &held {
            if selected && !read_only(class) && ui.input(|i| i.key_pressed(egui::Key::F2)) {
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

    /// The in-place editor, prefilled and selected. Enter or a click elsewhere settles
    /// it; Escape leaves the name alone.
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
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.rename = None;
            return None;
        }
        if !output.response.lost_focus() {
            return None;
        }
        let typed = std::mem::take(&mut rename.text);
        self.rename = None;
        renamed(original, &typed)
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

/// One row of the tree, sensing clicks and drags over its whole width.
fn row(ui: &mut egui::Ui, selected: bool, contents: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    let background = ui.painter().add(egui::Shape::Noop);
    let response = ui
        .scope_builder(
            egui::UiBuilder::new().sense(egui::Sense::click_and_drag()),
            |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    contents(ui);
                });
            },
        )
        .response;
    let fill = match (selected, response.hovered()) {
        (true, _) => Some(ui.visuals().selection.bg_fill),
        (false, true) => Some(ui.visuals().faint_bg_color),
        (false, false) => None,
    };
    if let Some(fill) = fill {
        ui.painter().set(
            background,
            egui::epaint::RectShape::filled(response.rect, 3.0, fill),
        );
    }
    response
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

    /// A folder holds exactly the kind named after it.
    #[test]
    fn every_kind_knows_the_folder_it_belongs_in() {
        for class in BROWSED {
            assert_eq!(Kind::from_class(class).home(), Some(class), "{class:?}");
        }
        assert_eq!(Kind::Other.home(), None);
    }
}
