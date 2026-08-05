use topcoat::{
    Result,
    view::{component, view},
};

// A link component that styles all anchor tags in MDX content.
#[component]
pub async fn branded_link(href: &'static str, #[default] child: topcoat::view::View) -> Result {
    view! { <a class="branded-link" href=(href)>(child)</a> }
}

// A wrapper component that wraps all MDX content in a styled container.
#[component]
pub async fn page_wrapper(#[default] child: topcoat::view::View) -> Result {
    view! { <div class="max-w-3xl mx-auto px-4 py-8">(child)</div> }
}
