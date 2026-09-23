use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

/// The classes for the [`skeleton`] placeholder.
const SKELETON: StaticClass = class!("animate-pulse rounded-md bg-foreground/10");

/// A pulsing placeholder for content that is loading.
///
/// Set width, height, and rounding classes to match the expected content and
/// reduce layout movement when it arrives. Attributes are forwarded to the
/// `<div>`, and classes are appended.
///
/// ```ignore
/// view! {
///     <div class="flex flex-col gap-2">
///         skeleton(attrs: attributes! { class="h-4 w-32" })
///         skeleton(attrs: attributes! { class="h-4 w-full" })
///     </div>
/// }
/// ```
#[component]
pub async fn skeleton(#[default] mut attrs: Attributes) -> Result<impl View> {
    Ok(view! { <div class=(class!(SKELETON, attrs.remove("class"))) (attrs)></div> })
}
