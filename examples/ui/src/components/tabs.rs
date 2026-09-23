use topcoat::{
    Result,
    runtime::Expr,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// A set of panels selected by links.
///
/// Links can navigate to a server-rendered panel or use click handlers to
/// update a signal. For browser-side switching, bind each trigger's `active`
/// prop and each panel's `hidden` attribute to that signal. The links do not
/// provide the ARIA tab pattern's arrow-key navigation.
///
/// Attributes are forwarded to the `<div>`, and classes are appended.
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

/// A row containing the panel selection links.
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
