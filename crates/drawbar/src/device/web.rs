//! Browser link: the device chooser, and a pump that runs one command at a time.
//!
//! There is one thread, so the "worker" is a chain of `spawn_local` tasks: each takes
//! the transport out of the shared cell, runs one command, and puts it back before
//! starting the next. Holding a `RefCell` borrow across an `await` would panic the
//! moment the UI touched the same cell, so the transport is moved rather than borrowed.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::mpsc::Sender;

use eframe::egui;
use js_sys::Promise;
use nord_usb::transport::{web::WebUsbTransport, VENDOR_ID};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{UsbDevice, UsbDeviceFilter, UsbDeviceRequestOptions};

use super::worker::{self, Emit, Flow};
use super::{DeviceCard, DeviceCmd, DeviceEvent};

#[derive(Default)]
struct Inner {
    transport: Option<WebUsbTransport>,
    queue: VecDeque<DeviceCmd>,
    /// A command is running, so the transport is out of the cell.
    busy: bool,
}

pub struct Link {
    emit: Emit,
    inner: Rc<RefCell<Inner>>,
}

impl Link {
    pub fn new(ctx: egui::Context, events: Sender<DeviceEvent>) -> Link {
        Link {
            emit: Emit::new(events, ctx),
            inner: Rc::new(RefCell::new(Inner::default())),
        }
    }

    /// Open the chooser and, once a device comes back, claim its vendor interface.
    ///
    /// ⚠️ `requestDevice()` must be called while the click's transient user activation
    /// is still live. Awaiting anything first — even an already-resolved promise —
    /// spends it, and Chrome then rejects with `SecurityError`. So the promise is taken
    /// here, synchronously, and only awaited inside the spawned task.
    pub fn connect(&mut self) {
        let request = match request_device() {
            Ok(request) => request,
            Err(e) => {
                self.emit.send(DeviceEvent::ConnectFailed(describe(&e)));
                return;
            }
        };

        let emit = self.emit.clone();
        let inner = self.inner.clone();
        spawn_local(async move {
            let device = match JsFuture::from(request).await {
                Ok(device) => device,
                Err(e) => {
                    emit.send(DeviceEvent::ConnectFailed(format!(
                        "no device chosen: {}",
                        describe(&e)
                    )));
                    return;
                }
            };
            let card = DeviceCard {
                product: device
                    .product_name()
                    .unwrap_or_else(|| "unnamed device".into()),
                manufacturer: device.manufacturer_name(),
                vendor_id: device.vendor_id(),
                product_id: device.product_id(),
                serial: device.serial_number(),
            };
            match WebUsbTransport::open(device).await {
                Ok(transport) => {
                    inner.borrow_mut().transport = Some(transport);
                    emit.send(DeviceEvent::Connected(card));
                }
                Err(e) => emit.send(DeviceEvent::ConnectFailed(e.to_string())),
            }
            pump(&inner, &emit);
        });
    }

    pub fn disconnect(&mut self) {
        self.send(DeviceCmd::Disconnect);
    }

    pub fn send(&mut self, cmd: DeviceCmd) {
        self.inner.borrow_mut().queue.push_back(cmd);
        pump(&self.inner, &self.emit);
    }
}

/// Start the next queued command, if the transport is free.
fn pump(inner: &Rc<RefCell<Inner>>, emit: &Emit) {
    let (transport, cmd) = {
        let mut state = inner.borrow_mut();
        if state.busy || state.transport.is_none() {
            return;
        }
        let Some(cmd) = state.queue.pop_front() else {
            return;
        };
        state.busy = true;
        (state.transport.take(), cmd)
    };
    let Some(mut transport) = transport else {
        return;
    };

    let inner = inner.clone();
    let emit = emit.clone();
    spawn_local(async move {
        let flow = worker::run(&mut transport, cmd, &emit).await;
        if flow == Flow::Stop {
            // ⚠️ Release the interface, or Nord Sound Manager and the desktop backend
            // stay locked out for as long as the tab is open.
            if let Err(e) = transport.close().await {
                emit.send(DeviceEvent::OpFailed(e.to_string()));
            }
            let mut state = inner.borrow_mut();
            state.busy = false;
            state.queue.clear();
            emit.send(DeviceEvent::Disconnected);
            return;
        }
        {
            let mut state = inner.borrow_mut();
            state.transport = Some(transport);
            state.busy = false;
        }
        pump(&inner, &emit);
    });
}

fn request_device() -> Result<Promise<UsbDevice>, JsValue> {
    let usb = web_sys::window()
        .ok_or_else(|| JsValue::from_str("no window"))?
        .navigator()
        .usb();

    // Filtering by vendor alone: the chooser then lists any Clavia device, and the
    // vendor-interface check in `WebUsbTransport::open` is what rejects a wrong one.
    let filter = UsbDeviceFilter::new();
    filter.set_vendor_id(VENDOR_ID);
    Ok(usb.request_device(&UsbDeviceRequestOptions::new(&[filter])))
}

/// A rejected promise carries a `DOMException`, whose text is on the object rather than
/// reachable by downcasting to `Error`.
fn describe(err: &JsValue) -> String {
    let field = |k: &str| {
        js_sys::Reflect::get(err, &JsValue::from_str(k))
            .ok()
            .and_then(|v| v.as_string())
    };
    match (field("name"), field("message")) {
        (Some(name), Some(message)) => format!("{name}: {message}"),
        (Some(only), None) | (None, Some(only)) => only,
        (None, None) => err.as_string().unwrap_or_else(|| format!("{err:?}")),
    }
}
