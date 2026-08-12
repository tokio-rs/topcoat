use topcoat::{
    Result,
    view::{Attributes, StaticClass, class, component, view},
};

/// The classes for the [`skeleton`] placeholder.
///
/// The block is filled with the foreground color at low opacity, so it reads
/// as an absence of content in both color schemes, and pulses to mark the
/// wait as ongoing.
const SKELETON: StaticClass = class!("animate-pulse rounded-md bg-foreground/10");

/// A skeleton component: a pulsing block standing in for content that has not
/// arrived yet.
///
/// The skeleton has no size of its own. Give it the shape of what it stands
/// in for through the `attrs`, with width, height, and rounding classes; a
/// column of them mirroring the real layout keeps the page from jumping once
/// the content lands. The `attrs` are forwarded to the underlying `<div>`; a
/// `class` among them is appended to the computed classes.
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
pub async fn skeleton(#[default] mut attrs: Attributes) -> Result {
    view! { <div class=(class!(SKELETON, attrs.remove("class"))) (attrs)></div> }
}
