//! Facade types for DOM events. These exist purely so handler bodies inside
//! `expr!` can be type-checked by rustc; they are never constructed
//! server-side. The browser resolves field accesses against the real DOM
//! `Event` at runtime.

use std::marker::PhantomData;

use crate::{BoolSurrogate, F64Surrogate, OptionSurrogate, StringSurrogate, Surrogate, Surrogated};

/// A browser event passed to a runtime event handler.
///
/// Read event data and call event methods synchronously, before returning or
/// awaiting. Targets are live browser handles, not snapshots. Copy strings or
/// other ordinary values before using them in asynchronous work.
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
    /// The native `ToggleEvent.newState`, or `None` when the event lacks it.
    ///
    /// Native opening and closing use `"open"` and `"closed"`. Synthetic events
    /// can supply other strings, including an empty string, which stay `Some`.
    pub new_state: OptionSurrogate<String>,
    pub offset_x: F64Surrogate,
    pub offset_y: F64Surrogate,
    /// The native `ToggleEvent.oldState`, with the same optionality as `new_state`.
    pub old_state: OptionSurrogate<String>,
    pub page_x: F64Surrogate,
    pub page_y: F64Surrogate,
    pub pointer_id: F64Surrogate,
    pub pointer_type: StringSurrogate,
    /// The native related target, or `None` when null or absent.
    ///
    /// For focus entering an element this is the target losing focus. For focus
    /// leaving it this is the target gaining focus. The browser may withhold it.
    /// A present target need not be an element.
    pub related_target: OptionSurrogate<EventTarget>,
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

/// A live, browser-local event target, which need not be an element.
///
/// Missing string properties read as empty strings and missing `checked` reads
/// as false. Use this handle during synchronous event dispatch. It cannot be
/// serialized into a signal or procedure argument.
pub struct EventTarget {
    pub checked: BoolSurrogate,
    pub id: StringSurrogate,
    pub name: StringSurrogate,
    pub text_content: StringSurrogate,
    pub value: StringSurrogate,
}

impl Surrogated for EventTarget {
    type Surrogate = Self;

    fn into_surrogate(self) -> Self {
        self
    }
}

impl Surrogate for EventTarget {
    type Real = Self;

    fn into_real(self) -> Self {
        self
    }
}
