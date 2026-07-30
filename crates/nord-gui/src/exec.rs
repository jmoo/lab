//! Driving a future from a synchronous UI callback.
//!
//! `nord-usb` is async and deliberately runtime-agnostic — no `Send` bounds, no
//! executor — so each platform supplies its own way of getting an answer out of it.

/// Natively, block the frame on the future. Fine for a demo: every operation the UI
/// can reach is a handful of bulk transfers.
#[cfg(not(target_arch = "wasm32"))]
pub fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    nord_usb::block_on(fut)
}

/// In the browser there is nothing to block *on* — but the only transport that exists
/// there is the replay, whose futures are ready the first time they are polled, so a
/// single poll cycle is enough and never spins.
#[cfg(target_arch = "wasm32")]
pub fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    fn vtable() -> &'static RawWakerVTable {
        &RawWakerVTable::new(
            |_| RawWaker::new(std::ptr::null(), vtable()),
            |_| {},
            |_| {},
            |_| {},
        )
    }

    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), vtable())) };
    let mut cx = Context::from_waker(&waker);
    let mut fut = std::pin::pin!(fut);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
    }
}
