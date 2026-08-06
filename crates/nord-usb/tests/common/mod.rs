//! What every test target here needs.
//!
//! ⚠️ A rustc-visible module, not a test target — each target compiles its own copy, so
//! an item no single target uses is still live.
#![allow(dead_code)]

/// Minimal executor: drive one future to completion on the calling thread.
///
/// Nothing under test ever really pends — the transports are a replayed script or an
/// in-memory responder — so a busy poll is exact and keeps tokio out of the dependency
/// tree of a crate that is deliberately runtime-agnostic.
pub fn block_on<F: std::future::Future>(mut fut: F) -> F::Output {
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
    let mut fut = unsafe { std::pin::Pin::new_unchecked(&mut fut) };
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

/// Hex text to bytes. Panics on anything that is not an even-length hex string, which is
/// what a mistyped capture line is.
pub fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
