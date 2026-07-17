use topcoat::{
    Result,
    view::{Attributes, View, class, component, view},
};

/// The classes for the [`label`] element.
///
/// The label lays out its content in a centered row, so an inline icon or a
/// wrapped control lines up with the text. It dims and stops receiving
/// pointer events when its control is disabled: a wrapped control is matched
/// with `has-[:disabled]`, a preceding sibling control marked `peer` with
/// `peer-disabled`.
const LABEL: &str = "flex items-center gap-2 text-sm leading-none font-medium select-none \
    peer-disabled:pointer-events-none peer-disabled:opacity-50 \
    has-[:disabled]:pointer-events-none has-[:disabled]:opacity-50";

/// A caption for a form control, rendered as a `<label>`.
///
/// Associate it with a control either by wrapping the control or by pointing
/// a `for` attribute at the control's `id`. The `attrs` (such as `class` or
/// `for`) are forwarded to the underlying `<label>`; a `class` among them is
/// appended to the computed classes. Child nodes become the label's content.
///
/// ```rust
/// view! {
///     <div class="flex flex-col gap-2">
///         label(attrs: attributes! { for="email" }, "Email")
///         input(attrs: attributes! { id="email" type="email" })
///     </div>
/// }
/// ```
#[component]
pub async fn label(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! { <label class=(class!(LABEL, attrs.remove("class"))) (attrs)>(child)</label> }
}
