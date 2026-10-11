use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

use super::floating::{FloatingAlign, FloatingAnchor, FloatingSide, floating_variants};

/// A floating action menu controlled by a trigger.
///
/// Pass a [`dropdown_menu_trigger`] and a [`dropdown_menu_content`] as children, and
/// connect them by passing the content's `id` to the trigger's `target`. Create the id
/// with [`element_id`](topcoat::view::element_id). The browser opens and closes the menu
/// without JavaScript. Clicking outside the menu or pressing Escape closes it. The menu
/// shows above the rest of the page, so containers that clip their overflow do not cut
/// it off.
///
/// `attrs` are forwarded to the wrapper `<div>`, with extra classes added to its classes.
/// The wrapper does not generate a box of its own. Items use normal Tab navigation. The
/// component does not implement the ARIA menu pattern's arrow-key navigation.
///
/// ```ignore
/// let menu = element_id(cx);
/// view! {
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
        <div class=(class!("group/dropdown-menu contents", attrs.remove("class"))) (attrs)>
            (child)
        </div>
    })
}

/// A button that opens or closes the dropdown menu.
///
/// Set `target` to the `id` of the [`dropdown_menu_content`]. Pass its label as
/// children. To style it as a button, pass classes from
/// [`button_variants`](super::button::button_variants) through `attrs`. The attributes
/// go on the `<button>`. Use `group-has-[:popover-open]/dropdown-menu:` classes to style
/// children while the menu is open.
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
    /// The `id` of the menu to open.
    target: &str,
    /// Extra attributes for the `<button>` element.
    #[default]
    mut attrs: Attributes,
    /// The trigger's label.
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
/// `side` chooses the side of the trigger it opens on and defaults to `Bottom`. `align`
/// lines it up with the trigger and defaults to `Center`. When the panel would overflow
/// the viewport, it moves to another position.
#[component]
pub async fn dropdown_menu_content(
    /// The panel's `id`, referenced by its trigger's `target`.
    id: &str,
    /// The side of the trigger the panel opens on.
    #[default]
    side: FloatingSide,
    /// How the panel lines up with the trigger.
    #[default]
    align: FloatingAlign,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The menu's items.
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

/// One action in a [`dropdown_menu_content`], rendered as a `<button type="button">`.
///
/// Set `close` to the menu's `id` to close the menu when the item is clicked. Closing a
/// menu also closes its submenus. Leave it unset for items that keep the menu open.
/// Submit buttons ignore `close`, which suits items that leave the page.
#[component]
pub async fn dropdown_menu_item(
    /// The `id` of the menu to close when the item is clicked.
    #[default]
    close: Option<&str>,
    /// Extra attributes for the `<button>` element.
    #[default]
    mut attrs: Attributes,
    /// The item's label.
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
/// Pass a [`dropdown_menu_sub_trigger`] and a [`dropdown_menu_sub_content`] as children,
/// and connect them by passing the content's `id` to the trigger's `target`. Opening the
/// submenu keeps the parent menu open, and closing the parent also closes the submenu.
///
/// `attrs` are forwarded to the wrapper `<div>`, with extra classes added to its classes.
/// The wrapper does not generate a box of its own.
///
/// ```ignore
/// let menu = element_id(cx);
/// let move_to = element_id(cx);
/// view! {
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
        <div class=(class!("group/dropdown-menu-sub contents", attrs.remove("class"))) (attrs)>
            (child)
        </div>
    })
}

/// The row that opens or closes a submenu.
///
/// Set `target` to the `id` of the [`dropdown_menu_sub_content`]. Pass its label as
/// children. A chevron points toward the submenu, and the row stays highlighted while it
/// is open. `attrs` are forwarded to the `<button>`, with extra classes added to its
/// classes.
#[component]
pub async fn dropdown_menu_sub_trigger(
    /// The `id` of the submenu to open.
    target: &str,
    /// Extra attributes for the `<button>` element.
    #[default]
    mut attrs: Attributes,
    /// The row's label.
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

/// The submenu panel, opening to the right of its trigger row.
///
/// When the panel would overflow the viewport, it moves to another position.
#[component]
pub async fn dropdown_menu_sub_content(
    /// The panel's `id`, referenced by its trigger's `target`.
    id: &str,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The submenu's items.
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
