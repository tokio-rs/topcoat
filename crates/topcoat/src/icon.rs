#![doc = include_str!("../docs/icon.md")]

pub use topcoat_icon::*;

#[cfg(feature = "icon-iconify")]
pub mod iconify {
    pub use topcoat_icon_iconify::*;
    pub use topcoat_icon_iconify_macro::*;
}

/// Renders an [`IconData`] as an inline `<svg>` element.
///
/// The icon is `1em` square by default, so it scales with the surrounding
/// text, and carries `aria-hidden` unless a `label` is passed.
#[topcoat::view::component]
pub async fn icon(
    /// The icon to render.
    data: IconData,
    /// The rendered width and height.
    #[into]
    #[default(::topcoat::view::Length::em(1.0))]
    size: topcoat::view::Length,
    /// An accessible label. When omitted, the icon is hidden from assistive
    /// technology.
    #[default]
    #[into]
    label: String,
    /// Extra attributes for the `<svg>` element.
    #[default]
    attrs: topcoat::view::Attributes,
) -> topcoat::Result {
    topcoat::view::view! {
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
