use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The classes for the [`label`] element.
const LABEL: StaticClass = class!(
    "flex items-center gap-2 text-sm leading-none font-medium select-none \
     peer-disabled:pointer-events-none peer-disabled:opacity-50 \
     peer-has-[:disabled]:pointer-events-none peer-has-[:disabled]:opacity-50 \
     has-[+:disabled]:pointer-events-none has-[+:disabled]:opacity-50 \
     has-[:disabled]:pointer-events-none has-[:disabled]:opacity-50",
);

/// A label for a form control.
///
/// Wrap the control or set `for` to its `id`. Attributes are forwarded to the
/// `<label>`, and classes are appended. Child content supplies the label.
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
