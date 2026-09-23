use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The classes for the [`textarea`] control.
///
/// The text size, radius, and focus ring match the input control.
/// `field-sizing-content` makes the control grow with its content, starting
/// at a two-line minimum height. Browsers without support keep the minimum
/// height and scroll.
const TEXTAREA: StaticClass = class!(
    "field-sizing-content min-h-16 w-full rounded-lg border border-border \
     bg-transparent px-3 py-2 text-sm transition-colors outline-none \
     placeholder:text-muted-foreground \
     focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
     aria-invalid:border-destructive aria-invalid:focus-visible:ring-destructive \
     focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50",
);

/// A multi-line text input.
///
/// The `attrs` (such as `name`, `placeholder`, `rows`, `disabled`, or event
/// handlers) are forwarded to the `<textarea>`. A `class` among them is
/// appended to the component's classes. Child nodes become the initial
/// value.
///
/// The textarea fills the width of its container, so size it through the
/// container or with a width class. Its height grows with its content,
/// starting at two lines. Set `aria-invalid="true"` to show the error border
/// and focus ring.
///
/// ```ignore
/// view! {
///     textarea(attrs: attributes! { name="feedback" placeholder="Tell us more" })
/// }
/// ```
#[component]
pub async fn textarea(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <textarea class=(class!(TEXTAREA, attrs.remove("class"))) (attrs)>
            (child)
        </textarea>
    })
}
