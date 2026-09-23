use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

/// The classes for the [`progress`] bar.
const PROGRESS: StaticClass = class!(
    "h-2 w-full appearance-none overflow-hidden rounded-full \
     bg-foreground/10 [&::-webkit-progress-bar]:bg-transparent \
     [&::-webkit-progress-value]:rounded-full [&::-webkit-progress-value]:bg-primary \
     [&::-webkit-progress-value]:transition-all \
     [&::-moz-progress-bar]:rounded-full [&::-moz-progress-bar]:bg-primary",
);

/// A themed native progress bar.
///
/// `value` is the completed amount out of `max`, which defaults to 100. Omit
/// `value` for indeterminate progress. Its appearance depends on the browser
/// and may be an empty track.
///
/// Attributes are forwarded to the `<progress>`, and classes are appended.
/// The bar fills its container. Use a width class to size it directly.
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
