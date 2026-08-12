use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

/// The classes for a [`kbd`] key cap.
///
/// The key is drawn like a physical cap: a tinted, hairline-bordered box just
/// wide enough for its label, and at least as wide as it is tall so that a
/// single character stays square. Browsers set a monospace family on `<kbd>`,
/// which `font-sans` takes back so the label matches the surrounding text.
const KBD: StaticClass = class!(
    "inline-flex h-5 w-fit min-w-5 shrink-0 items-center justify-center gap-1 \
     rounded-sm border border-border bg-foreground/5 px-1.5 font-sans text-xs font-medium \
     text-muted-foreground",
);

/// A keyboard key component: a `<kbd>` drawn as a key cap.
///
/// Child nodes become the key's label, a character or a key name. The `attrs`
/// (such as `class`) are forwarded to the `<kbd>`; a `class` among them is
/// appended to the computed classes. Group several keys of a shortcut with
/// [`kbd_group`].
///
/// ```ignore
/// view! {
///     kbd_group(kbd("Ctrl") kbd("K"))
/// }
/// ```
#[component]
pub async fn kbd(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! { <kbd class=(class!(KBD, attrs.remove("class"))) (attrs)>(child)</kbd> }
}

/// A row of [`kbd`] keys making up one shortcut.
///
/// The keys keep an even gap and stay on one line, so a chord reads as a
/// single unit next to the action it triggers.
#[component]
pub async fn kbd_group(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <span
            class=(class!(
                "inline-flex items-center gap-1 whitespace-nowrap",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </span>
    }
}
