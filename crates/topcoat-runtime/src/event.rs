//! Façade types for DOM events. These exist purely so handler bodies inside
//! `expr!` can be type-checked by rustc — they are never constructed
//! server-side. The browser resolves field accesses against the real DOM
//! `Event` at runtime.

use crate::String;

pub struct Event {
    pub target: EventTarget,
}

pub struct EventTarget {
    pub value: String,
}
