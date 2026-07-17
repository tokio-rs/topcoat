use topcoat::{
    Result,
    view::{Attributes, class, component, view},
};

/// The classes for the [`progress`] bar.
///
/// The element is a native `<progress>`, restyled through its
/// vendor-prefixed pseudo-elements so it looks the same across browsers: the
/// track is the element itself (the `::-webkit-progress-bar` layer is
/// cleared to let it show through), and the filled portion is rounded and
/// painted with the primary color. Browsers styled through the
/// `::-webkit-*` pseudo-elements only expose the filled portion while a
/// value is set, so the indeterminate state renders as a static empty track
/// there; Firefox keeps its animated bar.
const PROGRESS: &str = "h-2 w-full appearance-none overflow-hidden rounded-full \
    bg-foreground/10 [&::-webkit-progress-bar]:bg-transparent \
    [&::-webkit-progress-value]:rounded-full [&::-webkit-progress-value]:bg-primary \
    [&::-webkit-progress-value]:transition-all \
    [&::-moz-progress-bar]:rounded-full [&::-moz-progress-bar]:bg-primary";

/// A progress component: a themed native `<progress>` bar.
///
/// `value` is the completed amount, out of `max` (defaulting to 100, so a
/// plain value reads as a percentage). Omitting `value` renders the
/// indeterminate state, for work whose extent is unknown. The `attrs` (such
/// as `class` or `aria-label`) are forwarded to the `<progress>`; a `class`
/// among them is appended to the computed classes. The bar fills its
/// container, so size it through the container or with a width class.
///
/// ```rust
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
) -> Result {
    view! {
        <progress
            class=(class!(PROGRESS, attrs.remove("class")))
            value=(value)
            max=(max)
            (attrs)
        >

        </progress>
    }
}
