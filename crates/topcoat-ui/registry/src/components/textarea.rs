use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The classes for the [`textarea`] control.
const TEXTAREA: StaticClass = class!(
    "field-sizing-content min-h-16 w-full rounded-lg border border-border \
     bg-transparent px-3 py-2 text-sm transition-colors outline-none \
     placeholder:text-muted-foreground \
     focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
     aria-invalid:border-destructive aria-invalid:focus-visible:ring-destructive \
     focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50",
);

/// A multiline text input that grows with its content where supported.
///
/// Child content supplies the initial value. Attributes are forwarded to the
/// `<textarea>`, and classes are appended. It fills its container and starts at
/// a two-line minimum height. Use a width class to size it directly, or
/// `aria-invalid="true"` to show its error styling.
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
