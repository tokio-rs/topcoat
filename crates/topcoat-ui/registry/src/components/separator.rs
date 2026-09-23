use topcoat::{
    Result,
    view::{Attributes, PromotedStr, StaticClass, View, class, component, view},
};

/// The direction a [`separator`] runs in.
///
/// The default is `SeparatorOrientation::Horizontal`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SeparatorOrientation {
    /// A rule across the full width of its container.
    #[default]
    Horizontal,
    /// A rule down the full height of its container.
    Vertical,
}

impl SeparatorOrientation {
    /// The Tailwind classes for this orientation.
    ///
    /// The rule is one pixel thick and stretches in the other direction. A
    /// horizontal separator spans the width of its container, and a vertical
    /// one spans its height.
    fn classes(self) -> StaticClass {
        match self {
            Self::Horizontal => class!("h-px w-full"),
            Self::Vertical => class!("h-full w-px"),
        }
    }

    /// The value of the `aria-orientation` attribute. `None` for horizontal,
    /// which assistive technology assumes by default.
    fn aria(self) -> Option<PromotedStr> {
        match self {
            Self::Horizontal => None,
            Self::Vertical => Some(PromotedStr(&"vertical")),
        }
    }
}

/// The classes shared by both orientations.
///
/// The rule is drawn as a background instead of a border, so one set of
/// classes works for both orientations. The default border of `<hr>` is
/// removed. The rule never shrinks, so it stays visible in a crowded flex
/// row.
const SEPARATOR: StaticClass = class!("shrink-0 border-0 bg-border");

/// A thin line between groups of content.
///
/// The line is an `<hr>`, which assistive technology announces as a
/// separator. Its length comes from its container, so a vertical separator
/// needs a container with a height, such as a flex row whose items stretch.
/// `orientation` defaults to `Horizontal`.
///
/// The `attrs` (such as `class`) are forwarded to the `<hr>`. A `class` among
/// them is appended to the component's classes. To hide a purely decorative
/// line from assistive technology, add `aria-hidden="true"`.
///
/// ```ignore
/// view! {
///     <div class="flex flex-col gap-4">
///         <p>"Everyone with access to this workspace."</p>
///         separator()
///         <div class="flex h-5 items-center gap-3">
///             <a href="/docs">"Docs"</a>
///             separator(orientation: SeparatorOrientation::Vertical)
///             <a href="/blog">"Blog"</a>
///         </div>
///     </div>
/// }
/// ```
#[component]
pub async fn separator(
    /// The direction the rule runs in.
    #[default]
    orientation: SeparatorOrientation,
    /// Extra attributes for the `<hr>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <hr
            class=(class!(SEPARATOR, orientation.classes(), attrs.remove("class")))
            aria-orientation=(orientation.aria())
            (attrs)
        >
    })
}
