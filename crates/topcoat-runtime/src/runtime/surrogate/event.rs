//! Facade types for DOM events. These exist purely so handler bodies inside
//! `expr!` can be type-checked by rustc; they are never constructed
//! server-side. The browser resolves field accesses against the real DOM
//! `Event` at runtime.

use std::marker::PhantomData;

use crate::runtime::{BoolSurrogate, F64Surrogate, StringSurrogate};

pub struct Event {
    pub alt_key: BoolSurrogate,
    pub bubbles: BoolSurrogate,
    pub button: F64Surrogate,
    pub buttons: F64Surrogate,
    pub cancelable: BoolSurrogate,
    pub client_x: F64Surrogate,
    pub client_y: F64Surrogate,
    pub code: StringSurrogate,
    pub ctrl_key: BoolSurrogate,
    pub current_target: EventTarget,
    pub data: StringSurrogate,
    pub default_prevented: BoolSurrogate,
    pub delta_x: F64Surrogate,
    pub delta_y: F64Surrogate,
    pub delta_z: F64Surrogate,
    pub event_type: StringSurrogate,
    pub input_type: StringSurrogate,
    pub is_composing: BoolSurrogate,
    pub key: StringSurrogate,
    pub meta_key: BoolSurrogate,
    pub movement_x: F64Surrogate,
    pub movement_y: F64Surrogate,
    pub offset_x: F64Surrogate,
    pub offset_y: F64Surrogate,
    pub page_x: F64Surrogate,
    pub page_y: F64Surrogate,
    pub pointer_id: F64Surrogate,
    pub pointer_type: StringSurrogate,
    pub repeat: BoolSurrogate,
    pub screen_x: F64Surrogate,
    pub screen_y: F64Surrogate,
    pub shift_key: BoolSurrogate,
    pub target: EventTarget,
    pub time_stamp: F64Surrogate,
    _priv: PhantomData<()>,
}

impl Event {
    pub fn prevent_default(&self) {
        unreachable!();
    }

    pub fn stop_propagation(&self) {
        unreachable!();
    }

    pub fn stop_immediate_propagation(&self) {
        unreachable!();
    }
}

pub struct EventTarget {
    pub checked: BoolSurrogate,
    pub id: StringSurrogate,
    pub name: StringSurrogate,
    pub text_content: StringSurrogate,
    pub value: StringSurrogate,
}
