//! Internal support for compiler-generated Wasm bindings.

use serde::Serialize;

use crate::{Event, Signal, SignalSurrogate};

#[doc(hidden)]
pub fn __topcoat_wasm_root<F>(_: &str, expression: F) -> F {
    expression
}

#[doc(hidden)]
pub fn __wasm_event(_: Event) {}

#[doc(hidden)]
#[must_use]
pub fn __wasm_signal_capture<T>(signal: &Signal<T>) -> SignalSurrogate<T> {
    SignalSurrogate::new(signal.clone())
}

#[doc(hidden)]
#[derive(Serialize)]
pub struct WasmValue<T> {
    t: &'static str,
    v: T,
}

#[doc(hidden)]
pub fn __wasm_value<T>(v: T) -> WasmValue<T> {
    WasmValue { t: "Wasm", v }
}
