use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Length, View, attributes, class, component, view},
};

/// A spinning loader icon that shows that something is pending.
///
/// By default the spinner is `1em` square, so it scales with the surrounding
/// text and fits inline next to it. Pass `size` to set another size. `label`
/// is announced to assistive technology and defaults to `"Loading"`.
///
/// The `attrs` (such as `class`) are forwarded to the `<svg>`. A `class`
/// among them is appended to the component's classes.
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
