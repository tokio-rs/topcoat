use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Length, View, attributes, class, component, view},
};

/// An animated icon for work in progress.
///
/// The default size is `1em`, matching the surrounding text. Pass `size` to choose
/// other dimensions and `label` to describe the work to assistive technology. `attrs`
/// are forwarded to the `<svg>`, with extra classes added to its classes.
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
