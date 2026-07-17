use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Length, attributes, class, component, view},
};

/// A spinner component: a spinning loader icon for pending states.
///
/// The spinner is `1em` square by default, so it scales with the surrounding
/// text and sits inline next to it; pass `size` to set the dimensions
/// explicitly. The `attrs` (such as `class`) are forwarded to the underlying
/// `<svg>`; a `class` among them is appended to the computed classes.
///
/// ```rust
/// view! {
///     button(
///         attrs: attributes! { disabled=(true) },
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
) -> Result {
    view! {
        icon(
            data: iconify_icon!("feather:loader"),
            size: size,
            label: label,
            attrs: attributes! {
                class=(class!("animate-spin", attrs.remove("class")))
                (attrs)
            }
        )
    }
}
