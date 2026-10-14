use topcoat::{
    Result,
    view::{Attributes, PromotedStr, StaticClass, View, class, component, view},
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
    /// Classes that set the rule's thickness and stretch it along its orientation.
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

/// Classes for a separator that keeps its thickness in a flex layout.
const SEPARATOR: StaticClass = class!("shrink-0 border-0 bg-border");

/// A thin rule between groups of content.
///
/// Uses an `<hr>` element. Its length comes from its container, so a vertical separator
/// needs a container with a height. Pass `aria-hidden="true"` for a purely decorative
/// rule. `attrs` are forwarded to the `<hr>`, with extra classes added to its classes.
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
