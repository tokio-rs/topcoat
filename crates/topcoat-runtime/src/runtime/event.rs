//! Façade types for DOM events. These exist purely so handler bodies inside
//! `expr!` can be type-checked by rustc — they are never constructed
//! server-side. The browser resolves field accesses against the real DOM
//! `Event` at runtime.

use std::marker::PhantomData;

pub struct Event {
    pub target: EventTarget,
    _priv: PhantomData<()>,
}

impl Event {
    pub fn prevent_default(&self) {
        unreachable!();
    }

    pub fn stop_propagation(&self) {
        unreachable!();
    }
}

pub struct EventTarget {
    pub value: String,
}
