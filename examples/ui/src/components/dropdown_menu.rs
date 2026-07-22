use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, View, attributes, class, component, view},
};

/// A dropdown menu: a trigger that toggles a floating panel of actions.
///
/// Built on `<details>`, so it opens and closes without scripting: clicking
/// the [`dropdown_menu_trigger`] toggles the [`dropdown_menu_content`] panel.
/// Clicking outside does not close it; that behavior needs scripting. The
/// `attrs` (such as `class` or `open`) are forwarded to the underlying
/// `<details>`; a `class` among them is appended to the computed classes.
///
/// ```ignore
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

/// The classes making a `<summary>` a plain clickable trigger: the default
/// disclosure marker is hidden and the cursor marks it as interactive.
const TRIGGER: &str = "cursor-pointer list-none [&::-webkit-details-marker]:hidden";

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
/// ```ignore
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
        <summary class=(class!(TRIGGER, attrs.remove("class"))) (attrs)>
            (child)
        </summary>
    }
}

/// The classes shared by the [`dropdown_menu_content`] and
/// [`dropdown_menu_sub_content`] panels: a raised surface styled like a card;
/// `z-50` lifts it over later content. It sets its own background and text
/// color, so it reads the same on any ancestor.
const PANEL: &str = "absolute z-50 min-w-40 rounded-lg border border-border bg-background p-1 \
    text-foreground shadow-sm";

/// The floating panel of a [`dropdown_menu`], holding the menu's items.
///
/// The panel drops directly below the trigger, aligned to its left edge.
#[component]
pub async fn dropdown_menu_content(
    #[default] mut attrs: Attributes,
    #[default] child: View,
) -> Result {
    view! {
        <div
            class=(class!(PANEL, "top-full left-0 mt-1", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    }
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
pub async fn dropdown_menu_item(
    #[default] mut attrs: Attributes,
    #[default] child: View,
) -> Result {
    view! {
        <button class=(class!(ITEM, attrs.remove("class"))) (attrs)>(child)</button>
    }
}

/// A nested submenu placed among the items of a [`dropdown_menu_content`].
///
/// Like the [`dropdown_menu`] itself it is built on `<details>`, so clicking
/// the [`dropdown_menu_sub_trigger`] toggles its [`dropdown_menu_sub_content`]
/// panel without scripting. The submenu tracks its own open state under the
/// `group/sub` name, so the `group-open/sub:` variant targets it without
/// disturbing the enclosing menu's `group`. Closing the enclosing menu hides
/// an open submenu but does not close it, so it is open again the next time
/// the menu opens; resetting it needs scripting. The `attrs` are forwarded to
/// the underlying `<details>`; a `class` among them is appended to the
/// computed classes.
///
/// ```ignore
/// view! {
///     dropdown_menu_content(
///         dropdown_menu_item("Back")
///         dropdown_menu_sub(
///             dropdown_menu_sub_trigger("Move to")
///             dropdown_menu_sub_content(
///                 dropdown_menu_item("Inbox")
///                 dropdown_menu_item("Archive")
///             )
///         )
///     )
/// }
/// ```
#[component]
pub async fn dropdown_menu_sub(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <details class=(class!("group/sub relative", attrs.remove("class"))) (attrs)>
            (child)
        </details>
    }
}

/// The trigger row of a [`dropdown_menu_sub`]: a `<summary>` styled as a
/// [`dropdown_menu_item`] that toggles the submenu.
///
/// Child nodes become the row's label; a chevron pointing toward the submenu
/// is appended automatically, and the row stays tinted while the submenu is
/// open. The `attrs` are forwarded to the `<summary>`; a `class` among them is
/// appended to the computed classes.
#[component]
pub async fn dropdown_menu_sub_trigger(
    #[default] mut attrs: Attributes,
    #[default] child: View,
) -> Result {
    view! {
        <summary
            class=(class!(
                ITEM,
                TRIGGER,
                "group-open/sub:bg-foreground/5",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
            icon(
                data: iconify_icon!("feather:chevron-right"),
                attrs: attributes! { class="ml-auto size-4" }
            )
        </summary>
    }
}

/// The floating panel of a [`dropdown_menu_sub`], holding the submenu's items.
///
/// A submenu opens beside its trigger rather than below it: `left-full` places
/// it against the right edge of the parent panel, `top-0` lines its top up with
/// the trigger row, and `ml-1` leaves the same gap the menu keeps from its own
/// trigger.
#[component]
pub async fn dropdown_menu_sub_content(
    #[default] mut attrs: Attributes,
    #[default] child: View,
) -> Result {
    view! {
        <div
            class=(class!(PANEL, "top-0 left-full ml-1", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
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
