//! Types that describe DOM events to runtime expressions. They exist so that
//! event handler bodies type-check in Rust. They are never constructed on
//! the server, and the browser reads their fields from the real DOM event.

use std::marker::PhantomData;

use crate::{BoolSurrogate, F64Surrogate, StringSurrogate};

/// The DOM event passed to an event handler closure.
///
/// Each field reads the DOM property named in its documentation. A property
/// the event does not have reads as `false`, `0.0`, or an empty string, and
/// a number that is not finite reads as `0.0`.
///
/// ```rust
/// # use topcoat::{Result, context::Cx, runtime::{Event, signal}, view::*};
/// # #[component]
/// # async fn example(cx: &Cx) -> Result<impl View> {
/// let text = signal(cx, String::new);
///
/// Ok(view! {
///     <input @input=$(|e: Event| text.set(e.target.value))>
/// })
/// # }
/// ```
pub struct Event {
    /// Whether the Alt key was held (`altKey`).
    pub alt_key: BoolSurrogate,
    /// Whether the event bubbles up through the DOM (`bubbles`).
    pub bubbles: BoolSurrogate,
    /// The mouse button that changed state (`button`).
    pub button: F64Surrogate,
    /// The mouse buttons held down, as a bit mask (`buttons`).
    pub buttons: F64Surrogate,
    /// Whether the event can be canceled (`cancelable`).
    pub cancelable: BoolSurrogate,
    /// The horizontal pointer position in the viewport (`clientX`).
    pub client_x: F64Surrogate,
    /// The vertical pointer position in the viewport (`clientY`).
    pub client_y: F64Surrogate,
    /// The physical key pressed (`code`).
    pub code: StringSurrogate,
    /// Whether the Control key was held (`ctrlKey`).
    pub ctrl_key: BoolSurrogate,
    /// The element the handler is attached to (`currentTarget`).
    pub current_target: EventTarget,
    /// The inserted characters of an input event (`data`).
    pub data: StringSurrogate,
    /// Whether the default action was prevented (`defaultPrevented`).
    pub default_prevented: BoolSurrogate,
    /// The horizontal scroll amount of a wheel event (`deltaX`).
    pub delta_x: F64Surrogate,
    /// The vertical scroll amount of a wheel event (`deltaY`).
    pub delta_y: F64Surrogate,
    /// The scroll amount on the z axis of a wheel event (`deltaZ`).
    pub delta_z: F64Surrogate,
    /// The event's name, such as `"click"` (`type`).
    pub event_type: StringSurrogate,
    /// The kind of change of an input event (`inputType`).
    pub input_type: StringSurrogate,
    /// Whether the event happened during text composition (`isComposing`).
    pub is_composing: BoolSurrogate,
    /// The key value of the key pressed (`key`).
    pub key: StringSurrogate,
    /// Whether the Meta key was held (`metaKey`).
    pub meta_key: BoolSurrogate,
    /// The horizontal pointer movement since the last event (`movementX`).
    pub movement_x: F64Surrogate,
    /// The vertical pointer movement since the last event (`movementY`).
    pub movement_y: F64Surrogate,
    /// The horizontal pointer position within the target (`offsetX`).
    pub offset_x: F64Surrogate,
    /// The vertical pointer position within the target (`offsetY`).
    pub offset_y: F64Surrogate,
    /// The horizontal pointer position in the document (`pageX`).
    pub page_x: F64Surrogate,
    /// The vertical pointer position in the document (`pageY`).
    pub page_y: F64Surrogate,
    /// The id of the pointer that caused the event (`pointerId`).
    pub pointer_id: F64Surrogate,
    /// The kind of pointer, such as `"mouse"` or `"touch"` (`pointerType`).
    pub pointer_type: StringSurrogate,
    /// Whether the key is held down and repeating (`repeat`).
    pub repeat: BoolSurrogate,
    /// The horizontal pointer position on the screen (`screenX`).
    pub screen_x: F64Surrogate,
    /// The vertical pointer position on the screen (`screenY`).
    pub screen_y: F64Surrogate,
    /// Whether the Shift key was held (`shiftKey`).
    pub shift_key: BoolSurrogate,
    /// The element the event was dispatched to (`target`).
    pub target: EventTarget,
    /// The time the event was created, in milliseconds (`timeStamp`).
    pub time_stamp: F64Surrogate,
    _priv: PhantomData<()>,
}

impl Event {
    /// Prevents the browser's default action for the event
    /// (`preventDefault()`).
    pub fn prevent_default(&self) {
        unreachable!();
    }

    /// Stops the event from reaching further elements
    /// (`stopPropagation()`).
    pub fn stop_propagation(&self) {
        unreachable!();
    }

    /// Stops the event from reaching further elements and other handlers on
    /// the same element (`stopImmediatePropagation()`).
    pub fn stop_immediate_propagation(&self) {
        unreachable!();
    }
}

/// The element an [`Event`] refers to.
///
/// Like on [`Event`], a property the element does not have reads as `false`
/// or an empty string.
pub struct EventTarget {
    /// Whether a checkbox or radio button is checked (`checked`).
    pub checked: BoolSurrogate,
    /// The element's `id`.
    pub id: StringSurrogate,
    /// The element's `name`.
    pub name: StringSurrogate,
    /// The text content of the element (`textContent`).
    pub text_content: StringSurrogate,
    /// The current value of a form control (`value`).
    pub value: StringSurrogate,
}
