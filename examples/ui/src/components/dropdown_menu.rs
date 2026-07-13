use topcoat::{
    Result,
    view::{Attributes, View, class, component, view},
};

/// A dropdown menu: a trigger that toggles a floating panel of actions.
///
/// Built on `<details>`, so it opens and closes without scripting: clicking
/// the [`dropdown_menu_trigger`] toggles the [`dropdown_menu_content`] panel.
/// Clicking outside does not close it; that behavior needs scripting. The
/// `attrs` (such as `class` or `open`) are forwarded to the underlying
/// `<details>`; a `class` among them is appended to the computed classes.
///
/// ```rust
/// view! {
///     dropdown_menu(
///         dropdown_menu_trigger("Options")
///         dropdown_menu_content(
///             dropdown_menu_item("Rename")
///             dropdown_menu_item("Duplicate")
///             dropdown_menu_separator()
///             dropdown_menu_item(
///                 attrs: attributes! { class="text-destructive" },
///                 "Delete"
///             )
///         )
///     )
/// }
/// ```
#[component]
pub async fn dropdown_menu(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <details
            class=(class!("group relative inline-block", attrs.remove("class")))
            (attrs)
        >
            (child)
        </details>
    }
}

/// The trigger of a [`dropdown_menu`]: a `<summary>` that toggles the menu.
///
/// Child nodes become the trigger's content; any view works. The trigger
/// carries no styling of its own: dress it as a button by passing the classes
/// from [`button_variants`](super::button::button_variants), or leave it bare
/// for a custom look. While the menu is open the `group-open:` variant
/// applies within it, so a chevron with `group-open:rotate-180` flips along.
/// The `attrs` are forwarded to the `<summary>`; a `class` among them is
/// appended to the computed classes.
///
/// ```rust
/// view! {
///     dropdown_menu_trigger(
///         attrs: attributes! {
///             class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))
///         },
///         "Options"
///         icon(
///             data: iconify_icon!("feather:chevron-down"),
///             attrs: attributes! { class="group-open:rotate-180" }
///         )
///     )
/// }
/// ```
#[component]
pub async fn dropdown_menu_trigger(
    #[default] mut attrs: Attributes,
    #[default] child: View,
) -> Result {
    view! {
        <summary
            class=(class!(
                "cursor-pointer list-none [&::-webkit-details-marker]:hidden",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </summary>
    }
}

/// The classes for the [`dropdown_menu_content`] panel.
///
/// The panel drops directly below the trigger, aligned to its left edge, on a
/// raised surface styled like a card; `z-50` lifts it over later content. It
/// sets its own background and text color, so it reads the same on any
/// ancestor.
const CONTENT: &str = "absolute top-full left-0 z-50 mt-1 min-w-40 rounded-lg border \
    border-border bg-background p-1 text-foreground shadow-sm";

/// The floating panel of a [`dropdown_menu`], holding the menu's items.
#[component]
pub async fn dropdown_menu_content(
    #[default] mut attrs: Attributes,
    #[default] child: View,
) -> Result {
    view! { <div class=(class!(CONTENT, attrs.remove("class"))) (attrs)>(child)</div> }
}

/// The classes for a [`dropdown_menu_item`] row.
///
/// Hover, focus, and press tint the row like a ghost button, deriving the
/// states from the foreground color so they hold up in both color schemes.
const ITEM: &str = "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm \
    whitespace-nowrap outline-none hover:bg-foreground/5 focus-visible:bg-foreground/5 \
    active:bg-foreground/10 disabled:pointer-events-none disabled:opacity-50";

/// One action in a [`dropdown_menu_content`], rendered as a `<button>`.
#[component]
pub async fn dropdown_menu_item(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <button class=(class!(ITEM, attrs.remove("class"))) (attrs)>(child)</button>
    }
}

/// A non-interactive heading grouping the items after it.
#[component]
pub async fn dropdown_menu_label(
    #[default] mut attrs: Attributes,
    #[default] child: View,
) -> Result {
    view! {
        <p
            class=(class!(
                "px-2 py-1.5 text-xs font-medium text-muted-foreground",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </p>
    }
}

/// A hairline rule separating groups of items.
#[component]
pub async fn dropdown_menu_separator(#[default] mut attrs: Attributes) -> Result {
    view! {
        <hr class=(class!("-mx-1 my-1 border-border", attrs.remove("class"))) (attrs)>
    }
}
