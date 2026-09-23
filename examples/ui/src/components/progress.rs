use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

/// The classes for the [`progress`] bar.
///
/// The element is a native `<progress>`, styled through its vendor-prefixed
/// pseudo-elements so it looks the same in all browsers. The element itself
/// is the track, and the `::-webkit-progress-bar` layer is made transparent
/// so the track shows through. The filled part is rounded and uses the
/// primary color. Browsers that use the `::-webkit-*` pseudo-elements only
/// show the filled part while a value is set. In those browsers the
/// indeterminate state is an empty, static track. Firefox shows its own
/// animated bar.
const PROGRESS: StaticClass = class!(
    "h-2 w-full appearance-none overflow-hidden rounded-full \
     bg-foreground/10 [&::-webkit-progress-bar]:bg-transparent \
     [&::-webkit-progress-value]:rounded-full [&::-webkit-progress-value]:bg-primary \
     [&::-webkit-progress-value]:transition-all \
     [&::-moz-progress-bar]:rounded-full [&::-moz-progress-bar]:bg-primary",
);

/// A progress bar, rendered as a styled native `<progress>`.
///
/// `value` is the completed amount out of `max`. `max` defaults to 100, so
/// the value is a percentage by default. Without a `value`, the bar shows the
/// indeterminate state, for work of unknown length.
///
/// The `attrs` (such as `class` or `aria-label`) are forwarded to the
/// `<progress>`. A `class` among them is appended to the component's
/// classes. The bar fills the width of its container, so size it through the
/// container or with a width class.
///
/// ```ignore
/// view! {
///     progress(value: 62.0)
/// }
/// ```
#[component]
pub async fn progress(
    /// The completed amount, out of `max`.
    #[into]
    #[default]
    value: Option<f32>,
    /// The amount that counts as complete.
    #[default(100.0)]
    max: f32,
    /// Extra attributes for the `<progress>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <progress
            class=(class!(PROGRESS, attrs.remove("class")))
            value=(value)
            max=(max)
            (attrs)
        ></progress>
    })
}
