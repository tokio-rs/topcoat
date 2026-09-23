use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

/// The classes for the [`skeleton`] placeholder.
///
/// The block is filled with the foreground color at low opacity, so it works
/// in both color schemes. It pulses to show that content is still loading.
const SKELETON: StaticClass = class!("animate-pulse rounded-md bg-foreground/10");

/// A pulsing placeholder block for content that is still loading.
///
/// The skeleton has no size of its own. Give it the shape of the content it
/// replaces with width, height, and rounding classes in `attrs`. When the
/// skeletons follow the layout of the real content, the page does not jump
/// when the content arrives. The `attrs` are forwarded to the `<div>`, and a
/// `class` among them is appended to the component's classes.
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
