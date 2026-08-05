use topcoat::{
    Result,
    view::{View, component, view},
};

/// Renders a styled callout box. The `var` prop selects the callout variant
/// (e.g. `"info"`, `"warning"`, `"error"`), which becomes the CSS class suffix.
#[component]
pub async fn callout(var: &'static str, #[default] child: View) -> Result {
    view! { <div class=(format!("callout-{}", var))>(child)</div> }
}

/// Wraps its child content in a section element.
/// Useful for grouping related MDX content with shared styling.
#[component]
pub async fn wrapper(#[default] child: View) -> Result {
    view! { <section class="wrapper"><div class="wrapper-inner">(child)</div></section> }
}

/// Renders a horizontal rule divider.
/// Used as a self-closing component in MDX.
#[component]
pub async fn divider() -> Result {
    view! { <hr class="divider" /> }
}
