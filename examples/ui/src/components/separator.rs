use topcoat::{
    Result,
    view::{Attributes, PromotedStr, StaticClass, class, component, view},
};

/// The direction a [`separator`] runs in.
///
/// [`Default`] is `SeparatorOrientation::Horizontal`, used when no
/// orientation is given.
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
    /// The rule is a hairline in one direction and stretches along the other,
    /// so it takes its length from the container: a horizontal separator
    /// spans the container's width, a vertical one its height.
    fn classes(self) -> StaticClass {
        match self {
            Self::Horizontal => class!("h-px w-full"),
            Self::Vertical => class!("h-full w-px"),
        }
    }

    /// The value of the `aria-orientation` attribute, or `None` for the
    /// horizontal default assistive technology already assumes.
    fn aria(self) -> Option<PromotedStr> {
        match self {
            Self::Horizontal => None,
            Self::Vertical => Some(PromotedStr(&"vertical")),
        }
    }
}

/// The classes shared by both orientations.
///
/// The rule is painted as a background rather than a border, which is what
/// lets one set of classes cover both orientations; the border an `<hr>`
/// carries by default is cleared for it. It never shrinks, so it survives in
/// a crowded flex row.
const SEPARATOR: StaticClass = class!("shrink-0 border-0 bg-border");

/// A separator component: a hairline rule between groups of content.
///
/// The rule is an `<hr>`, which assistive technology already announces as a
/// separator. It takes its length from its container, so a vertical separator
/// needs a container that gives it a height, such as a flex row whose items
/// stretch. The `attrs` (such as `class`) are forwarded to the `<hr>`; a
/// `class` among them is appended to the computed classes. A rule that is
/// only decoration can be hidden from assistive technology with an
/// `aria-hidden` attribute among them.
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
) -> Result {
    view! {
        <hr
            class=(class!(SEPARATOR, orientation.classes(), attrs.remove("class")))
            aria-orientation=(orientation.aria())
            (attrs)
        >
    }
}
