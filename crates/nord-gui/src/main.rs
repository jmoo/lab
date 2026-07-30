//! `nord-gui` — a proof of concept: one egui application over [`nord_format`] and
//! [`nord_usb`], compiled both as a desktop binary and to wasm for the browser.
//!
//! Everything above the entry points below is platform-independent. The two places that
//! are not are isolated deliberately:
//!
//! - [`exec`] — how a future is driven (pollster natively, a busy-poll on wasm).
//! - [`device`] — which transports exist (USB is desktop-only; the replayed capture
//!   runs everywhere).
//!
//! Not a product, and not feature complete: it reads, it never writes. No mutating
//! device operation is reachable from this UI.

mod app;
mod demo;
mod device;
mod exec;
mod panel;
mod theme;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 840.0])
            .with_min_inner_size([760.0, 520.0])
            .with_title("nord-gui"),
        ..Default::default()
    };
    eframe::run_native(
        "nord-gui",
        options,
        Box::new(|cc| Ok(Box::new(app::NordGui::new(cc)))),
    )
}

/// The browser entry point. `index.html` supplies the canvas; wasm-bindgen calls `main`
/// when the module starts, and eframe drives the frame loop from `requestAnimationFrame`.
#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast as _;

    console_error_panic_hook::set_once();

    let canvas = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("nord_gui_canvas"))
        .expect("index.html must provide a #nord_gui_canvas element")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("#nord_gui_canvas is not a <canvas>");

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(app::NordGui::new(cc)))),
            )
            .await
            .expect("failed to start eframe");
    });
}
