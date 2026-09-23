use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The classes for a [`kbd`] key cap.
const KBD: StaticClass = class!(
    "inline-flex h-5 w-fit min-w-5 shrink-0 items-center justify-center gap-1 \
     rounded-sm border border-border bg-foreground/5 px-1.5 font-sans text-xs font-medium \
     text-muted-foreground",
);

/// A key name displayed as a key cap.
///
/// Child content supplies the label. Use `kbd_group` for a shortcut with several
/// keys. Attributes are forwarded to the `<kbd>`, and classes are appended.
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

/// A row of key names forming one keyboard shortcut.
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
