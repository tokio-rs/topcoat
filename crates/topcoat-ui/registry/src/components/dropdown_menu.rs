use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

/// A trigger that opens and closes a floating panel of actions.
///
/// The menu is a `<details>` element, so it works without scripting. Clicking
/// the [`dropdown_menu_trigger`] opens and closes the
/// [`dropdown_menu_content`] panel. Clicking outside the menu does not close
/// it. That needs scripting.
///
/// The `attrs` (such as `class` or `open`) are forwarded to the `<details>`.
/// A `class` among them is appended to the component's classes. The same
/// holds for the other dropdown menu components.
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
pub async fn dropdown_menu(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <details
            class=(class!("group relative inline-block", attrs.remove("class")))
            (attrs)
        >
            (child)
        </details>
    })
}

/// The classes that make a `<summary>` a plain trigger: the default marker is
/// hidden and the cursor is a pointer.
const TRIGGER: StaticClass = class!("cursor-pointer list-none [&::-webkit-details-marker]:hidden",);

/// The trigger of a [`dropdown_menu`]: a `<summary>` that opens and closes
/// the menu.
///
/// Child nodes become the trigger's content. The trigger has no style of its
/// own. To make it look like a button, pass the classes from
/// [`button_variants`](super::button::button_variants) in `attrs`. While the
/// menu is open, the `group-open:` variant applies inside it, so a chevron
/// with `group-open:rotate-180` flips when the menu opens.
///
/// ```ignore
/// view! {
///     dropdown_menu_trigger(
///         attrs: attributes! {
///             class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))
///         },
///         "Options"
///         icon(
///             data: iconify_icon!("lucide:chevron-down"),
///             attrs: attributes! { class="group-open:rotate-180" }
///         )
///     )
/// }
/// ```
#[component]
pub async fn dropdown_menu_trigger(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <summary class=(class!(TRIGGER, attrs.remove("class"))) (attrs)>
            (child)
        </summary>
    })
}

/// The classes shared by the [`dropdown_menu_content`] and
/// [`dropdown_menu_sub_content`] panels: a raised surface styled like a card.
/// `z-50` puts it above later content. It sets its own background and text
/// color, so it looks the same on any background.
const PANEL: StaticClass = class!(
    "absolute z-50 min-w-40 rounded-lg border border-border bg-popover p-1 \
     text-popover-foreground shadow-sm",
);

/// The floating panel of a [`dropdown_menu`], holding the menu's items.
///
/// The panel opens below the trigger, aligned to its left edge.
#[component]
pub async fn dropdown_menu_content(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(PANEL, "top-full left-0 mt-1", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// The classes for a [`dropdown_menu_item`] row.
///
/// Hover, focus, and press tint the row like a ghost button. The tints use
/// the foreground color, so they work in both color schemes.
const ITEM: StaticClass = class!(
    "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm \
     whitespace-nowrap outline-none hover:bg-foreground/5 focus-visible:bg-foreground/5 \
     active:bg-foreground/10 disabled:pointer-events-none disabled:opacity-50",
);

/// One action in a [`dropdown_menu_content`], rendered as a `<button>`.
#[component]
pub async fn dropdown_menu_item(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <button class=(class!(ITEM, attrs.remove("class"))) (attrs)>(child)</button>
    })
}

/// A nested submenu among the items of a [`dropdown_menu_content`].
///
/// Like the [`dropdown_menu`], it is a `<details>` element. Clicking the
/// [`dropdown_menu_sub_trigger`] opens and closes its
/// [`dropdown_menu_sub_content`] panel without scripting. The submenu is a
/// Tailwind group named `sub`, so the `group-open/sub:` variant refers to the
/// submenu and not to the outer menu. Closing the outer menu hides an open
/// submenu but does not close it, so the submenu is still open the next time
/// the menu opens. Closing it as well needs scripting.
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
pub async fn dropdown_menu_sub(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <details class=(class!("group/sub relative", attrs.remove("class"))) (attrs)>
            (child)
        </details>
    })
}

/// The trigger row of a [`dropdown_menu_sub`]: a `<summary>` styled like a
/// [`dropdown_menu_item`] that opens and closes the submenu.
///
/// Child nodes become the row's label. A chevron that points toward the
/// submenu is added after them. The row stays tinted while the submenu is
/// open.
#[component]
pub async fn dropdown_menu_sub_trigger(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
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
                data: iconify_icon!("lucide:chevron-right"),
                attrs: attributes! { class="ml-auto size-4" }
            )
        </summary>
    })
}

/// The floating panel of a [`dropdown_menu_sub`], holding the submenu's items.
///
/// The submenu opens next to its trigger, not below it. `left-full` places it
/// at the right edge of the parent panel, `top-0` aligns its top with the
/// trigger row, and `ml-1` leaves the same gap as between the menu and its
/// trigger.
#[component]
pub async fn dropdown_menu_sub_content(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(PANEL, "top-0 left-full ml-1", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// A heading for the items that follow it, rendered as a `<p>`.
#[component]
pub async fn dropdown_menu_label(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <p
            class=(class!(
                "px-2 py-1.5 text-xs font-medium text-muted-foreground",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </p>
    })
}

/// A thin line between groups of items, rendered as an `<hr>`.
#[component]
pub async fn dropdown_menu_separator(#[default] mut attrs: Attributes) -> Result<impl View> {
    Ok(view! {
        <hr class=(class!("-mx-1 my-1 border-border", attrs.remove("class"))) (attrs)>
    })
}
