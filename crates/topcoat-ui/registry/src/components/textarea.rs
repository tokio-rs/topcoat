use topcoat::{
    Result,
    view::{Attributes, View, class, component, view},
};

/// The classes for the [`textarea`] control.
///
/// The text size, radius, shadow, and focus ring match the input control.
/// `field-sizing-content` lets the control grow with its content, from the
/// two-line minimum height; browsers without support keep the fixed minimum
/// and scroll.
const TEXTAREA: &str = "field-sizing-content min-h-16 w-full rounded-lg border border-border \
    bg-background px-3 py-2 text-sm shadow-xs transition-colors outline-none \
    placeholder:text-muted-foreground \
    focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
    focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50";

/// A multi-line text input component.
///
/// The `attrs` (such as `name`, `placeholder`, `rows`, `disabled`, or event
/// handlers) are forwarded to the underlying `<textarea>`; a `class` among
/// them is appended to the computed classes. Child nodes become the control's
/// initial value. The textarea fills its container, so size it through the
/// container or with a width class; it grows with its content from a
/// two-line minimum.
///
/// ```rust
/// view! {
///     textarea(attrs: attributes! { name="feedback" placeholder="Tell us more" })
/// }
/// ```
#[component]
pub async fn textarea(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <textarea class=(class!(TEXTAREA, attrs.remove("class"))) (attrs)>
            (child)
        </textarea>
    }
}
