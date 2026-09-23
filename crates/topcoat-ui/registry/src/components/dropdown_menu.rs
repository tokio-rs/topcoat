use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

/// A menu that opens and closes when its trigger is clicked.
///
/// It uses `<details>` and needs no scripting to toggle. Closing on an outside
/// click requires scripting. Attributes are forwarded to the `<details>`, and
/// classes are appended.
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

/// The classes making a `<summary>` a plain clickable trigger: the default
/// disclosure marker is hidden and the cursor marks it as interactive.
const TRIGGER: StaticClass = class!("cursor-pointer list-none [&::-webkit-details-marker]:hidden",);

/// A `<summary>` that toggles the menu.
///
/// Child content supplies the trigger. Add classes for its appearance, such as
/// those returned by [`button_variants`](super::button::button_variants).
/// The `group-open:` variant applies while the menu is open.
/// Attributes are forwarded to the `<summary>`, and classes are appended.
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
/// [`dropdown_menu_sub_content`] panels: a raised surface styled like a card;
/// `z-50` lifts it over later content. It sets its own background and text
/// color, so it reads the same on any ancestor.
const PANEL: StaticClass = class!(
    "absolute z-50 min-w-40 rounded-lg border border-border bg-popover p-1 \
     text-popover-foreground shadow-sm",
);

/// The floating panel of a [`dropdown_menu`], holding the menu's items.
///
/// The panel drops directly below the trigger, aligned to its left edge.
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

/// A submenu that toggles independently using `<details>`.
///
/// Closing the parent menu hides the submenu but preserves its open state.
/// Use scripting to reset it when the parent closes. The `group-open/sub:`
/// variant targets this submenu. Attributes are forwarded to the `<details>`,
/// and classes are appended.
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

/// A row that toggles a submenu.
///
/// Child content supplies its label. A chevron is added automatically.
/// Attributes are forwarded to the `<summary>`, and classes are appended.
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

/// A submenu panel positioned to the right of its trigger.
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

/// A non-interactive heading grouping the items after it.
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

/// A hairline rule separating groups of items.
#[component]
pub async fn dropdown_menu_separator(#[default] mut attrs: Attributes) -> Result<impl View> {
    Ok(view! {
        <hr class=(class!("-mx-1 my-1 border-border", attrs.remove("class"))) (attrs)>
    })
}
