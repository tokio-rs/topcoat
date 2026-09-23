use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The classes for the [`label`] element.
///
/// The label lays out its content in a centered row, so an inline icon or a
/// wrapped control lines up with the text. The label dims and ignores the
/// pointer when its control is disabled. These selectors find the control:
///
/// - `has-[:disabled]`: a control inside the label.
/// - `peer-disabled`: a preceding sibling control marked `peer`.
/// - `peer-has-[:disabled]`: a control inside a preceding `peer` wrapper.
/// - `has-[+:disabled]`: a control right after the label.
const LABEL: StaticClass = class!(
    "flex items-center gap-2 text-sm leading-none font-medium select-none \
     peer-disabled:pointer-events-none peer-disabled:opacity-50 \
     peer-has-[:disabled]:pointer-events-none peer-has-[:disabled]:opacity-50 \
     has-[+:disabled]:pointer-events-none has-[+:disabled]:opacity-50 \
     has-[:disabled]:pointer-events-none has-[:disabled]:opacity-50",
);

/// A caption for a form control, rendered as a `<label>`.
///
/// Connect it to a control by putting the control inside the label, or by
/// setting `for` to the control's `id`. The `attrs` (such as `class` or
/// `for`) are forwarded to the `<label>`. A `class` among them is appended to
/// the component's classes. Child nodes become the label's content.
///
/// ```ignore
/// view! {
///     <div class="flex flex-col gap-2">
///         label(attrs: attributes! { for="email" }, "Email")
///         input(attrs: attributes! { id="email" type="email" })
///     </div>
/// }
/// ```
#[component]
pub async fn label(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <label class=(class!(LABEL, attrs.remove("class"))) (attrs)>(child)</label>
    })
}
