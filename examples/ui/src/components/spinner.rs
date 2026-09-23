use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Length, View, attributes, class, component, view},
};

/// A spinning icon for a pending operation.
///
/// Its default size of `1em` follows the surrounding text. Set `size` for
/// explicit dimensions. Attributes are forwarded to the `<svg>`, and classes
/// are appended.
///
/// ```ignore
/// view! {
///     button(
///         attrs: attributes! { disabled="" },
///         spinner()
///         "Saving..."
///     )
/// }
/// ```
#[component]
pub async fn spinner(
    /// The rendered width and height.
    #[into]
    #[default(Length::em(1.0))]
    size: Length,
    /// The label announced to assistive technology.
    #[into]
    #[default(String::from("Loading"))]
    label: String,
    /// Extra attributes for the `<svg>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        icon(
            data: iconify_icon!("lucide:loader-circle"),
            size: size,
            label: label,
            attrs: attributes! {
                class=(class!("animate-spin", attrs.remove("class")))
                (attrs)
            }
        )
    })
}
