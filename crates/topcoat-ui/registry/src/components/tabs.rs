use topcoat::{
    Result,
    runtime::Expr,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// Several panels that show one at a time, selected by a row of triggers
/// above them.
///
/// A trigger can link to a page that renders its panel on the server, or
/// update a signal in a click handler. To switch panels in the browser, bind
/// each trigger's `active` prop and each panel's `hidden` attribute to a
/// signal that holds the selected tab. Triggers are plain links and do not
/// support the arrow key navigation of the ARIA tab pattern.
///
/// The `attrs` (such as `class`) are forwarded to the `<div>`. A `class`
/// among them is appended to the component's classes. The same holds for the
/// other tabs components.
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
/// The triggers sit in a bordered box, like the toggles in a toggle group,
/// so the tabs look like one control.
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
/// The active trigger is tinted and uses the foreground color. The other
/// triggers are muted and change color on hover.
const TRIGGER: StaticClass = class!(
    "inline-flex shrink-0 cursor-pointer items-center justify-center gap-2 \
     rounded-md px-3 py-1.5 text-sm font-medium whitespace-nowrap transition-colors outline-none \
     focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
     focus-visible:ring-offset-background text-muted-foreground \
     hover:bg-foreground/5 hover:text-foreground \
     aria-[current=page]:bg-foreground/10 aria-[current=page]:text-foreground \
     aria-[current=page]:hover:bg-foreground/10",
);

/// A trigger in a [`tabs_list`]: a link to the page that shows its panel.
///
/// `active` accepts a boolean or a runtime expression. It controls the
/// active style and `aria-current="page"`, and defaults to `false`. Pass the
/// `href` in `attrs`. A click handler in the browser can prevent the
/// navigation and select the panel by updating a signal instead.
#[component]
pub async fn tabs_trigger(
    /// Whether this trigger's panel is the one shown.
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

/// The panel of the active [`tabs_trigger`].
///
/// Render only the selected panel on the server, or render all panels with
/// `:hidden` bindings to switch them in the browser.
#[component]
pub async fn tabs_content(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! { <div class=(attrs.remove("class")) (attrs)>(child)</div> })
}
