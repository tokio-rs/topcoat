use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, StaticClass, View, attributes, class, component, view},
};

/// The classes for the native `<input type="checkbox">` inside the
/// [`checkbox`] component.
///
/// `appearance-none` hides the native checkmark, so the component can draw
/// its own and look the same in all browsers. The unchecked box has the same
/// border as the input control. When checked, the box is filled with the
/// primary color.
const CHECKBOX: StaticClass = class!(
    "peer size-4 shrink-0 appearance-none rounded-[4px] border border-border \
     bg-background transition-colors outline-none \
     checked:border-primary checked:bg-primary \
     focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
     focus-visible:ring-offset-background disabled:pointer-events-none",
);

/// A checkbox, rendered as a styled native `<input type="checkbox">`.
///
/// The `attrs` (such as `name`, `checked`, `disabled`, or event handlers) are
/// forwarded to the `<input>`. A `class` among them is not put on the input
/// but appended to the classes of the `<span>` that wraps it. Set the checked
/// state with a `checked` attribute. The indeterminate state has no style of
/// its own, because it can only be set from a script.
///
/// ```ignore
/// view! {
///     <div class="flex items-center gap-2">
///         checkbox(attrs: attributes! { id="terms" name="terms" checked="" })
///         label(attrs: attributes! { for="terms" }, "Accept terms")
///     </div>
/// }
/// ```
#[component]
pub async fn checkbox(#[default] mut attrs: Attributes) -> Result<impl View> {
    // The `<input>` cannot draw the checkmark, because it renders no children
    // or pseudo-elements. So the checkmark is a sibling icon placed over the
    // control, and the input's `peer` state shows it while checked.
    Ok(view! {
        <span
            class=(class!(
                "peer relative inline-flex shrink-0 has-[:disabled]:opacity-50",
                attrs.remove("class"),
            ))
        >
            <input type="checkbox" class=(CHECKBOX) (attrs)>
            icon(
                data: iconify_icon!("lucide:check"),
                attrs: attributes! {
                    class="pointer-events-none absolute inset-0 m-auto size-3.5 \
                        text-primary-foreground opacity-0 peer-checked:opacity-100"
                }
            )
        </span>
    })
}
