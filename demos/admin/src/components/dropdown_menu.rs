use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

use super::floating::{FloatingAlign, FloatingAnchor, FloatingSide, floating_variants};

/// A floating action menu controlled by a trigger.
///
/// Add a [`dropdown_menu_trigger`] and a [`dropdown_menu_content`] as children. Create
/// an ID with [`element_id`](topcoat::view::element_id) and use it for the trigger's
/// `target` and the content's `id`. The button opens and closes the menu without
/// JavaScript. Clicking outside or pressing Escape also closes it. The menu appears
/// above the page and is not cut off by a parent's overflow settings.
///
/// `attrs` adds attributes and classes to the wrapper `<div>`, which takes up no space
/// in the layout. Use Tab to move between items. Arrow-key menu navigation is not
/// provided.
///
/// ```ignore
/// view! {
///     let menu = element_id(cx);
///     dropdown_menu(
///         dropdown_menu_trigger(target: &menu, "Options")
///         dropdown_menu_content(
///             id: &menu,
///             align: FloatingAlign::Start,
///             dropdown_menu_item(close: Some(&menu), "Rename")
///             dropdown_menu_item(close: Some(&menu), "Duplicate")
///             dropdown_menu_separator()
///             dropdown_menu_item(
///                 close: Some(&menu),
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
        <div
            class=(class!("group/dropdown-menu contents", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// A button for opening and closing the dropdown menu.
///
/// Set `target` to the [`dropdown_menu_content`]'s `id` and put the button's label in
/// its children. Add [`button_variants`](super::button::button_variants) classes through
/// `attrs` to give it the standard button style. All attributes go on the `<button>`.
/// Use `group-has-[:popover-open]/dropdown-menu:` classes to change how children look
/// while the menu is open.
///
/// ```ignore
/// view! {
///     dropdown_menu_trigger(
///         target: &menu,
///         attrs: attributes! {
///             class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))
///         },
///         "Options"
///         icon(
///             data: iconify_icon!("lucide:chevron-down"),
///             attrs: attributes! {
///                 class="group-has-[:popover-open]/dropdown-menu:rotate-180"
///             }
///         )
///     )
/// }
/// ```
#[component]
pub async fn dropdown_menu_trigger(
    /// The menu ID this button controls.
    target: &str,
    /// Attributes to add to the button.
    #[default]
    mut attrs: Attributes,
    /// Content shown inside the button.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <button
            type="button"
            popovertarget=(target)
            class=(class!("cursor-pointer", attrs.remove("class")))
            (attrs)
        >
            (child)
        </button>
    })
}

/// Classes for floating menu panels with their own background, border, and text color.
const PANEL: StaticClass = class!(
    "min-w-40 rounded-lg border border-border bg-popover p-1",
    "text-popover-foreground shadow-sm outline-none",
);

/// The floating panel of a [`dropdown_menu`], holding the menu's items.
///
/// Use `side` to choose where the menu opens and `align` to line it up with the button.
/// By default, it opens below the button and is centered on it. The browser tries other
/// positions if there is not enough room in the window.
#[component]
pub async fn dropdown_menu_content(
    /// The menu ID to pass to the trigger's `target`.
    id: &str,
    /// Where the menu opens relative to the button.
    #[default]
    side: FloatingSide,
    /// Which edge or center of the button the menu lines up with.
    #[default]
    align: FloatingAlign,
    /// Attributes to add to the menu's `<div>`.
    #[default]
    mut attrs: Attributes,
    /// Items shown inside the menu.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            id=(id)
            popover="auto"
            class=(class!(
                floating_variants(FloatingAnchor::Invoker, side, align),
                PANEL,
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}

/// Classes for a menu item and its interaction states.
const ITEM: StaticClass = class!(
    "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm",
    "whitespace-nowrap outline-none hover:bg-foreground/5 focus-visible:bg-foreground/5",
    "active:bg-foreground/10 disabled:pointer-events-none disabled:opacity-50",
    "[&_svg]:size-4 [&_svg]:shrink-0",
);

/// A button for an action inside [`dropdown_menu_content`].
///
/// The button defaults to `type="button"`. Pass the menu's ID to `close` to close the
/// menu and its submenus when clicked. Omit `close` to leave them open. If you set
/// `type="submit"` through `attrs`, the button submits its form and ignores `close`.
#[component]
pub async fn dropdown_menu_item(
    /// The menu ID to close when this button is clicked.
    #[default]
    close: Option<&str>,
    /// Attributes to add to the button.
    #[default]
    mut attrs: Attributes,
    /// Content shown inside the item.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <button
            type="button"
            popovertarget=(close)
            popovertargetaction=(close.map(|_| "hide"))
            class=(class!(ITEM, attrs.remove("class")))
            (attrs)
        >
            (child)
        </button>
    })
}

/// A nested menu that opens from a row in the parent menu.
///
/// Add a [`dropdown_menu_sub_trigger`] and a [`dropdown_menu_sub_content`] as children.
/// Give the trigger's `target` and the content's `id` the same value. The parent menu
/// stays open when the submenu opens. Closing the parent closes the submenu too.
///
/// `attrs` adds attributes and classes to the wrapper `<div>`. The wrapper takes up no
/// space in the layout.
///
/// ```ignore
/// view! {
///     let menu = element_id(cx);
///     let move_to = element_id(cx);
///     dropdown_menu_content(
///         id: &menu,
///         dropdown_menu_item(close: Some(&menu), "Back")
///         dropdown_menu_sub(
///             dropdown_menu_sub_trigger(target: &move_to, "Move to")
///             dropdown_menu_sub_content(
///                 id: &move_to,
///                 dropdown_menu_item(close: Some(&menu), "Inbox")
///                 dropdown_menu_item(close: Some(&menu), "Archive")
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
        <div
            class=(class!("group/dropdown-menu-sub contents", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// The row that opens or closes a submenu.
///
/// Set `target` to the [`dropdown_menu_sub_content`]'s `id` and put the label in the
/// children. The row includes a chevron and stays highlighted while the submenu is
/// open. `attrs` adds attributes and classes to the row's `<button>`.
#[component]
pub async fn dropdown_menu_sub_trigger(
    /// The submenu ID this button controls.
    target: &str,
    /// Attributes to add to the button.
    #[default]
    mut attrs: Attributes,
    /// Content shown inside the row.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <button
            type="button"
            popovertarget=(target)
            class=(class!(
                ITEM,
                "group-has-[:popover-open]/dropdown-menu-sub:bg-foreground/5",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
            icon(
                data: iconify_icon!("lucide:chevron-right"),
                attrs: attributes! { class="ml-auto" }
            )
        </button>
    })
}

/// A submenu that opens to the right of its button.
///
/// The browser tries other positions if the submenu does not fit in the window.
#[component]
pub async fn dropdown_menu_sub_content(
    /// The submenu ID to pass to the trigger's `target`.
    id: &str,
    /// Attributes to add to the submenu's `<div>`.
    #[default]
    mut attrs: Attributes,
    /// Items shown inside the submenu.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            id=(id)
            popover="auto"
            class=(class!(
                floating_variants(
                    FloatingAnchor::Invoker,
                    FloatingSide::Right,
                    FloatingAlign::Start,
                ),
                PANEL,
                attrs.remove("class"),
            ))
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
