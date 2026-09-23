use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The classes for a [`kbd`] key cap.
///
/// The key looks like a physical key cap: a tinted box with a thin border,
/// just wide enough for its label. It is at least as wide as it is tall, so a
/// single character stays square. Browsers give `<kbd>` a monospace font, and
/// `font-sans` resets it so the label matches the surrounding text.
const KBD: StaticClass = class!(
    "inline-flex h-5 w-fit min-w-5 shrink-0 items-center justify-center gap-1 \
     rounded-sm border border-border bg-foreground/5 px-1.5 font-sans text-xs font-medium \
     text-muted-foreground",
);

/// A keyboard key, rendered as a `<kbd>` that looks like a key cap.
///
/// Child nodes become the key's label, such as a character or a key name.
/// The `attrs` (such as `class`) are forwarded to the `<kbd>`. A `class`
/// among them is appended to the component's classes. To show a shortcut
/// made of several keys, put them in a [`kbd_group`].
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

/// A row of [`kbd`] keys that make up one shortcut.
///
/// The keys have an even gap and stay on one line. Child nodes become the
/// row's content. The `attrs` are forwarded to the `<span>`, and a `class`
/// among them is appended to the component's classes.
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
