use topcoat::{
    Result,
    runtime::Expr,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// A tabs component: one panel at a time out of several, picked by the row of
/// triggers above it.
///
/// Triggers can navigate to a server-rendered panel or update a signal in a
/// click handler. For browser-side switching, bind each trigger's `active`
/// prop and each panel's `hidden` attribute to the selected-tab signal.
/// Triggers are ordinary links, without the ARIA tab pattern's arrow-key
/// navigation.
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
pub async fn tabs(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class=(class!("flex flex-col gap-4", attrs.remove("class"))) (attrs)>
            (child)
        </div>
    })
}

/// The row of triggers at the top of a [`tabs`].
///
/// The triggers sit in a rail, the same one a toggle group uses, so a set of
/// tabs reads as one control rather than as loose links.
#[component]
pub async fn tabs_list(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "inline-flex w-fit items-center gap-1 rounded-lg border border-border p-1",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
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
     focus-visible:ring-offset-background text-muted-foreground \
     hover:bg-foreground/5 hover:text-foreground \
     aria-[current=page]:bg-foreground/10 aria-[current=page]:text-foreground \
     aria-[current=page]:hover:bg-foreground/10",
);

/// One trigger of a [`tabs_list`]: a link to the page showing its panel.
///
/// `active` accepts a boolean or a runtime expression and controls the
/// selected styling and `aria-current="page"`. Pass the `href` among the
/// `attrs`. A client-side click handler can prevent navigation and select a
/// panel by updating a signal instead.
#[component]
pub async fn tabs_trigger(
    /// Whether this trigger's panel is the one on show.
    #[into]
    #[default(false.into())]
    active: Expr<bool>,
    /// Extra attributes for the `<a>` element.
    #[default]
    mut attrs: Attributes,
    /// The trigger's label.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <a
            :aria-current=$(active.then_some("page"))
            class=(class!(TRIGGER, attrs.remove("class")))
            (attrs)
        >
            (child)
        </a>
    })
}

/// The panel of the [`tabs_trigger`] on show.
///
/// Render only the selected panel on the server, or render all panels with
/// `:hidden` bindings for browser-side switching.
#[component]
pub async fn tabs_content(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! { <div class=(attrs.remove("class")) (attrs)>(child)</div> })
}
