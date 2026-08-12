use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

/// A tabs component: one panel at a time out of several, picked by the row of
/// triggers above it.
///
/// Which panel shows is server state: a [`tabs_trigger`] is a link, and the
/// page renders the [`tabs_content`] for whichever one the URL names. The
/// panel is therefore shareable and survives a reload, at the cost of a
/// navigation when switching. Because the triggers are ordinary links, this
/// is a navigation rather than the ARIA tab pattern, which expects the arrow
/// keys to move between tabs and needs scripting.
///
/// The `attrs` (such as `class`) are forwarded to the underlying `<div>`; a
/// `class` among them is appended to the computed classes.
///
/// ```ignore
/// view! {
///     tabs(
///         tabs_list(
///             for (value, text) in TABS {
///                 tabs_trigger(
///                     active: value == tab,
///                     attrs: attributes! { href=(format!("?tab={value}")) },
///                     (text)
///                 )
///             }
///         )
///         tabs_content((panel))
///     )
/// }
/// ```
#[component]
pub async fn tabs(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <div class=(class!("flex flex-col gap-4", attrs.remove("class"))) (attrs)>
            (child)
        </div>
    }
}

/// The row of triggers at the top of a [`tabs`].
///
/// The triggers sit in a rail, the same one a toggle group uses, so a set of
/// tabs reads as one control rather than as loose links.
#[component]
pub async fn tabs_list(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <div
            class=(class!(
                "inline-flex w-fit items-center gap-1 rounded-lg border border-border p-1 \
                 shadow-xs",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    }
}

/// The classes for a [`tabs_trigger`].
///
/// The trigger for the panel on show is tinted and takes the full foreground
/// color, which is what tells it apart from the rest; the others light up on
/// hover.
const TRIGGER: StaticClass = class!(
    "inline-flex shrink-0 cursor-pointer items-center justify-center gap-2 \
     rounded-md px-3 py-1.5 text-sm font-medium whitespace-nowrap transition-colors outline-none \
     focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
     focus-visible:ring-offset-background",
);

/// One trigger of a [`tabs_list`]: a link to the page showing its panel.
///
/// `active` marks the trigger whose panel is on show, which also carries
/// `aria-current="page"` so assistive technology announces where the reader
/// is. Pass the `href` among the `attrs`.
#[component]
pub async fn tabs_trigger(
    /// Whether this trigger's panel is the one on show.
    #[default]
    active: bool,
    /// Extra attributes for the `<a>` element.
    #[default]
    mut attrs: Attributes,
    /// The trigger's label.
    #[default]
    child: View,
) -> Result {
    let state = if active {
        "bg-foreground/10 text-foreground"
    } else {
        "text-muted-foreground hover:bg-foreground/5 hover:text-foreground"
    };

    view! {
        <a
            aria-current=(active.then_some("page"))
            class=(class!(TRIGGER, state, attrs.remove("class")))
            (attrs)
        >
            (child)
        </a>
    }
}

/// The panel of the [`tabs_trigger`] on show.
///
/// Only the panel being shown is rendered, so there is one of these on the
/// page at a time and it needs no state of its own.
#[component]
pub async fn tabs_content(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! { <div class=(attrs.remove("class")) (attrs)>(child)</div> }
}
