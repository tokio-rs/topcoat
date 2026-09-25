use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

/// A floating action menu controlled by a trigger.
///
/// Uses a native `<details>` element, so the trigger opens and closes it without
/// JavaScript. Closing it on an outside click requires application scripting. `attrs`
/// are forwarded to the `<details>`, with extra classes added to its classes.
/// Items use normal Tab navigation. The component does not implement the ARIA menu
/// pattern's arrow-key navigation.
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

/// Classes that hide the native disclosure marker and show a pointer cursor.
const TRIGGER: StaticClass = class!("cursor-pointer list-none [&::-webkit-details-marker]:hidden",);

/// A trigger that opens or closes the dropdown menu.
///
/// Pass its label as children. To style it as a button, pass classes from
/// [`button_variants`](super::button::button_variants) through `attrs`. The attributes
/// go on the `<summary>`. Use `group-open:` classes to style children while the menu is
/// open.
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

/// Classes for floating menu panels with their own background, border, and text color.
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

/// Classes for a menu item and its interaction states.
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

/// A nested menu that opens from a row in the parent menu.
///
/// Its trigger toggles a native `<details>` element without JavaScript. Closing the
/// parent hides the submenu but preserves its open state. Resetting that state requires
/// application scripting.
///
/// Use `group-open/sub:` classes to style children while the submenu is open. `attrs`
/// are forwarded to the `<details>`, with extra classes added to its classes.
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

/// The row that opens or closes a submenu.
///
/// Pass its label as children. A chevron points toward the submenu, and the row stays
/// highlighted while it is open. `attrs` are forwarded to the `<summary>`, with extra
/// classes added to its classes.
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

/// The submenu panel, positioned to the right of its trigger row.
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
