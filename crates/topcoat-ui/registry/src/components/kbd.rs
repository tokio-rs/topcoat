use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// Classes that draw a key label in a bordered box.
const KBD: StaticClass = class!(
    "inline-flex h-5 w-fit min-w-5 shrink-0 items-center justify-center gap-1 \
     rounded-sm border border-border bg-foreground/5 px-1.5 font-sans text-xs font-medium \
     text-muted-foreground",
);

/// A keyboard key label rendered as `<kbd>`.
///
/// Pass the key name as children and use [`kbd_group`] for a shortcut with several
/// keys. `attrs` are forwarded to the `<kbd>`, with extra classes added to its classes.
///
/// ```ignore
/// view! {
///     kbd_group(kbd("Ctrl") kbd("K"))
/// }
/// ```
#[component]
pub async fn kbd(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! { <kbd class=(class!(KBD, attrs.remove("class"))) (attrs)>(child)</kbd> })
}

/// A row of key labels for one keyboard shortcut. The keys stay on one line.
#[component]
pub async fn kbd_group(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span
            class=(class!(
                "inline-flex items-center gap-1 whitespace-nowrap",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </span>
    })
}
