use topcoat::{
    Result,
    view::{Attributes, class, component, view},
};

/// The classes for the [`input`] control.
///
/// The height, text size, radius, shadow, and focus ring match the `Md`
/// button, so an input and a button sit flush in a row. File inputs restyle
/// the browser's upload button into quiet, borderless text.
const INPUT: &str = "h-9 w-full min-w-0 rounded-lg border border-border bg-background px-3 \
    text-sm shadow-xs transition-colors outline-none \
    placeholder:text-muted-foreground \
    file:mr-3 file:h-full file:border-0 file:bg-transparent file:text-sm file:font-medium \
    focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
    focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50";

/// A text input component.
///
/// The `attrs` (such as `type`, `name`, `placeholder`, `disabled`, or event
/// handlers) are forwarded to the underlying `<input>`; a `class` among them
/// is appended to the computed classes. The input fills its container, so
/// size it through the container or with a width class.
///
/// ```ignore
/// view! {
///     input(attrs: attributes! { type="email" placeholder="you@example.com" })
/// }
/// ```
#[component]
pub async fn input(#[default] mut attrs: Attributes) -> Result {
    view! { <input class=(class!(INPUT, attrs.remove("class"))) (attrs)> }
}
