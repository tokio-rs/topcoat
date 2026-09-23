use topcoat_core::error::Result;
use topcoat_view::{Attributes, Length, View};
use topcoat_view_macro::{component, view};

use crate::IconData;

/// Renders an [`IconData`] as an inline `<svg>` element.
///
/// The icon is `1em` square by default, so it scales with the font size of the
/// surrounding text. It is shifted down slightly to line up with the text.
///
/// Without a `label`, the icon is marked `aria-hidden` so assistive technology
/// skips it. With a `label`, it gets `role="img"` and the label as its
/// `aria-label`.
#[component]
pub async fn icon(
    /// The icon to render.
    data: IconData,
    /// The rendered width and height.
    #[into]
    #[default(Length::em(1.0))]
    size: Length,
    /// The accessible name of the icon. When empty, the icon is hidden from
    /// assistive technology.
    #[default]
    #[into]
    label: String,
    /// Extra attributes for the `<svg>` element.
    #[default]
    attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <svg
            viewBox=(data.view_box())
            width=(size)
            height=(size)
            style="vertical-align: -0.125em"
            aria-hidden=(label.is_empty().then_some("true"))
            role=((!label.is_empty()).then_some("img"))
            aria-label=((!label.is_empty()).then_some(label))
            (attrs)
        >
            (data.into_body())
        </svg>
    })
}
