use topcoat_core::error::Result;
use topcoat_view::View;
use topcoat_view_macro::{component, view};

use crate::{ControlValue, FormControl};

/// Renders a labelled text-like input bound to a [`FormControl`].
///
/// The generated `<input>` carries the `data-control-*` attributes the live
/// update handlers read, and renders the first validation error after the field.
/// Use `input_type` to switch to `email`, `password`, `url`, etc.
#[component]
pub async fn text_input<T: ControlValue>(
    control: &FormControl<T>,
    name: &str,
    label: &str,
    #[default("text")] input_type: &str,
) -> Result<View> {
    let value = control.value().to_control_string();
    let error = control.errors().first().map(|e| e.message().to_owned());

    view! {
        <label for=(name)>(label)</label>
        <input
            type=(input_type)
            id=(name)
            name=(name)
            value=(value.as_str())
            data-control-name=(name)
            data-control-value=(value.as_str())
            data-control-dirty=(if control.dirty() { "true" } else { "false" })
            data-control-touched=(if control.touched() { "true" } else { "false" })
        >
        if let Some(error) = error {
            <span style="color: red;">(error)</span>
        }
        <br>
    }
}

/// Renders a labelled number input bound to a [`FormControl`].
///
/// This is a convenience wrapper around [`text_input`] with `type="number"`.
#[component]
pub async fn number_input<T: ControlValue>(
    control: &FormControl<T>,
    name: &str,
    label: &str,
) -> Result<View> {
    view! {
        text_input(
            control: control,
            name: name,
            label: label,
            input_type: "number",
        )
    }
}

/// Renders a labelled checkbox bound to a [`FormControl<bool>`].
#[component]
pub async fn checkbox(control: &FormControl<bool>, name: &str, label: &str) -> Result<View> {
    let checked = *control.value();
    let error = control.errors().first().map(|e| e.message().to_owned());

    view! {
        <label>
            <input
                type="checkbox"
                name=(name)
                value="true"
                if checked { checked="" }
                data-control-name=(name)
                data-control-value=(if checked { "true" } else { "false" })
                data-control-dirty=(if control.dirty() { "true" } else { "false" })
                data-control-touched=(if control.touched() { "true" } else { "false" })
            >
            (label)
        </label>
        if let Some(error) = error {
            <span style="color: red;">(error)</span>
        }
        <br>
    }
}

/// Renders a labelled textarea bound to a [`FormControl`].
#[component]
pub async fn textarea<T: ControlValue>(
    control: &FormControl<T>,
    name: &str,
    label: &str,
) -> Result<View> {
    let value = control.value().to_control_string();
    let error = control.errors().first().map(|e| e.message().to_owned());

    view! {
        <label for=(name)>(label)</label>
        <textarea
            id=(name)
            name=(name)
            data-control-name=(name)
            data-control-value=(value.as_str())
            data-control-dirty=(if control.dirty() { "true" } else { "false" })
            data-control-touched=(if control.touched() { "true" } else { "false" })
        >(value.as_str())</textarea>
        if let Some(error) = error {
            <span style="color: red;">(error)</span>
        }
        <br>
    }
}
