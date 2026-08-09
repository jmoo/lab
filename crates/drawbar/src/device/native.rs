//! Desktop link: `nusb` enumeration, and a thread that owns the transport.
//!
//! The worker is a plain thread blocking on its command channel. Nothing about the
//! protocol is concurrent — one transaction at a time — so a thread that runs one
//! command to completion and then waits is the whole scheduler.

use std::sync::mpsc::{self, Sender};

use eframe::egui;
use nord_usb::transport::{usb, UsbTransport};

use super::worker::{self, Emit, Flow};
use super::{DeviceCard, DeviceCmd, DeviceEvent};

pub struct Link {
    ctx: egui::Context,
    events: Sender<DeviceEvent>,
    /// `None` while disconnected. Dropping it is what ends the worker thread.
    commands: Option<Sender<DeviceCmd>>,
}

impl Link {
    pub fn new(ctx: egui::Context, events: Sender<DeviceEvent>) -> Link {
        Link {
            ctx,
            events,
            commands: None,
        }
    }

    pub fn connect(&mut self) {
        let (tx, rx) = mpsc::channel::<DeviceCmd>();
        self.commands = Some(tx);
        let emit = Emit::new(self.events.clone(), self.ctx.clone());

        std::thread::spawn(move || {
            let mut transport = match open() {
                Ok((card, transport)) => {
                    emit.send(DeviceEvent::Connected(card));
                    transport
                }
                Err(why) => {
                    emit.send(DeviceEvent::ConnectFailed(why));
                    return;
                }
            };
            // `recv` ends when the UI drops its sender, so a disconnect that races the
            // thread still stops it.
            while let Ok(cmd) = rx.recv() {
                if nord_usb::block_on(worker::run(&mut transport, cmd, &emit)) == Flow::Stop {
                    break;
                }
            }
            // Dropping the transport releases the claimed interface, which is what lets
            // Nord Sound Manager and nord-cli have the device back.
            drop(transport);
            emit.send(DeviceEvent::Disconnected);
        });
    }

    pub fn disconnect(&mut self) {
        // Sent, then the sender is dropped: the queued command still arrives, and the
        // closed channel ends the loop even if it does not.
        if let Some(tx) = self.commands.take() {
            let _ = tx.send(DeviceCmd::Disconnect);
        }
    }

    pub fn send(&mut self, cmd: DeviceCmd) {
        if let Some(tx) = &self.commands {
            let _ = tx.send(cmd);
        }
    }
}

/// The first attached Clavia, with the descriptor facts the card shows.
///
/// The vendor-interface check happens here rather than inside the first transaction:
/// a Clavia this tool cannot drive should say so at connect time, not fail somewhere
/// inside a session.
fn open() -> Result<(DeviceCard, UsbTransport), String> {
    let devices = usb::list().map_err(|e| e.to_string())?;
    let info = devices
        .into_iter()
        .next()
        .ok_or("no Clavia device found — is the instrument awake and on a data cable?")?;

    if !info.interfaces().any(|i| i.class() == 0xff) {
        return Err(format!(
            "{} exposes no vendor interface; this tool cannot drive it",
            info.product_string().unwrap_or("the attached device"),
        ));
    }

    let card = DeviceCard {
        product: info
            .product_string()
            .unwrap_or("unnamed device")
            .to_string(),
        manufacturer: info.manufacturer_string().map(str::to_string),
        vendor_id: info.vendor_id(),
        product_id: info.product_id(),
        serial: info.serial_number().map(str::to_string),
    };
    let transport = UsbTransport::open(&info).map_err(|e| e.to_string())?;
    Ok((card, transport))
}
