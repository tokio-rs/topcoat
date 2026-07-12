use topcoat_core::error::Result;
use topcoat_view::{Attributes, Length, View};
use topcoat_view_macro::{component, view};

use crate::IconData;

/// Renders an [`IconData`] as an inline `<svg>` element.
///
/// The icon is `1em` square by default, so it scales with the surrounding
/// text, and carries `aria-hidden` unless a `label` is passed.
#[component]
pub async fn icon(
    /// The icon to render.
    data: IconData,
    /// The rendered width and height.
    #[into]
    #[default(Length::em(1.0))]
    size: Length,
    /// An accessible label. When omitted, the icon is hidden from assistive
    /// technology.
    #[default]
    #[into]
    label: String,
    /// Extra attributes for the `<svg>` element.
    #[default]
    attrs: Attributes,
) -> Result<View> {
    view! {
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
    }
}
