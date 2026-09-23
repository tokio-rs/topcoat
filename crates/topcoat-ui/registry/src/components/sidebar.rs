use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    runtime::Expr,
    view::{Attributes, Child, Class, StaticClass, View, attributes, class, component, view},
};

use super::{
    button::{ButtonSize, ButtonVariant, button},
    input::input,
    separator::separator,
    sheet::{SheetSide, sheet, sheet_content},
    skeleton::skeleton,
};

/// The edge of the page that a [`sidebar`] sits on.
///
/// The default is `SidebarSide::Left`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SidebarSide {
    /// The left edge.
    #[default]
    Left,
    /// The right edge. Place the sidebar after the page content.
    Right,
}

impl SidebarSide {
    /// The value of the `data-side` attribute.
    fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    /// The side the mobile sheet comes in from.
    fn sheet(self) -> SheetSide {
        match self {
            Self::Left => SheetSide::Left,
            Self::Right => SheetSide::Right,
        }
    }
}

/// How the desktop [`sidebar`] looks next to the page.
///
/// The default is `SidebarVariant::Sidebar`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SidebarVariant {
    /// A panel separated from the page by a border.
    #[default]
    Sidebar,
    /// A rounded panel with space around it.
    Floating,
    /// A sidebar next to page content in a rounded [`sidebar_inset`] surface.
    Inset,
}

impl SidebarVariant {
    /// The value of the `data-variant` attribute.
    fn name(self) -> &'static str {
        match self {
            Self::Sidebar => "sidebar",
            Self::Floating => "floating",
            Self::Inset => "inset",
        }
    }
}

/// What the desktop [`sidebar`] does when its `open` expression is false.
///
/// The default is `SidebarCollapsible::Offcanvas`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SidebarCollapsible {
    /// Hides the panel and gives its space to the page.
    #[default]
    Offcanvas,
    /// Shrinks the panel to a narrow strip of menu icons.
    Icon,
    /// Keeps the desktop panel expanded, whatever `open` is.
    None,
}

impl SidebarCollapsible {
    /// The value of the `data-collapsible` attribute while collapsed.
    fn name(self) -> &'static str {
        match self {
            Self::Offcanvas => "offcanvas",
            Self::Icon => "icon",
            Self::None => "none",
        }
    }
}

/// The flex layout around a [`sidebar`] and the page content.
///
/// On desktop, the layout is as high as the viewport, and the page content
/// scrolls inside it.
///
/// To change the widths, set `--sidebar-width`, `--sidebar-width-mobile`, and
/// `--sidebar-width-icon` in `attrs`, for example with a class like
/// `[--sidebar-width:20rem]`. Place a right sidebar after the page content.
/// The components keep no state. You pass expressions to [`sidebar`] and
/// event handlers to [`sidebar_trigger`] and [`sidebar_rail`].
///
/// The `attrs` are forwarded to the `<div>`. A `class` among them is appended
/// to the component's classes. Unless noted otherwise, the other sidebar
/// components also forward their `attrs` to their element and append a
/// `class`.
#[component]
pub async fn sidebar_provider(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            data-sidebar="provider"
            class=(class!(
                "flex min-h-svh w-full md:h-svh md:overflow-hidden [--sidebar-width:16rem] [--sidebar-width-mobile:18rem] [--sidebar-width-icon:3rem] md:has-[[data-variant=inset]]:bg-sidebar",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}

// Other components inside the panel use the sidebar palette too. These
// scoped aliases give buttons, inputs, and captions the sidebar colors
// instead of the page colors.
const PANEL_THEME: StaticClass = class!(
    "[--background:var(--sidebar)] [--foreground:var(--sidebar-foreground)] \
     [--card:var(--sidebar)] [--card-foreground:var(--sidebar-foreground)] \
     [--primary:var(--sidebar-primary)] \
     [--primary-foreground:var(--sidebar-primary-foreground)] \
     [--border:var(--sidebar-border)] [--ring:var(--sidebar-ring)] \
     [--muted-foreground:color-mix(in_oklab,var(--sidebar-foreground)_70%,transparent)]",
);

/// A navigation panel at the edge of the page, which becomes a [`sheet`]
/// over the page on screens narrower than `md` (48rem).
///
/// The `--sidebar-*` theme tokens set its colors, separately from cards,
/// sheets, and the page. Components inside the sidebar use the sidebar
/// colors too.
///
/// `open` controls whether the desktop panel is expanded, and defaults to
/// `true`. `mobile_open` controls the mobile sheet separately and defaults to
/// `false`, so the sheet can start closed while the desktop panel starts
/// expanded. Pass runtime expressions that read your signals to open and
/// close them in the browser. The children, usually a [`sidebar_header`], a
/// [`sidebar_content`], and a [`sidebar_footer`], are rendered once and used
/// for both screen sizes, so inputs and signals inside them keep their state.
///
/// `side`, `variant`, and `collapsible` default to `Left`, `Sidebar`, and
/// `Offcanvas`. The `attrs` are forwarded to the outer `<aside>`. The
/// `sheet_attrs` are forwarded to the `<dialog>` of the sheet, for example a
/// label, an id, or event handlers that close the sheet on Escape or on a
/// click on the overlay. Add a close button to the mobile header. Like
/// [`sheet`], the mobile sheet does not trap focus. That needs extra
/// scripting.
#[component]
pub async fn sidebar(
    #[into]
    #[default(true.into())]
    open: Expr<bool>,
    #[into]
    #[default(false.into())]
    mobile_open: Expr<bool>,
    #[default] side: SidebarSide,
    #[default] variant: SidebarVariant,
    #[default] collapsible: SidebarCollapsible,
    #[default] mut attrs: Attributes,
    #[default] mut sheet_attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let open = if collapsible == SidebarCollapsible::None {
        true.into()
    } else {
        open
    };
    let collapse = collapsible.name();

    Ok(view! {
        <aside
            data-sidebar="sidebar"
            data-side=(side.name())
            data-variant=(variant.name())
            :data-state=$(if open { "expanded" } else { "collapsed" })
            :data-collapsible=$(if open { "" } else { collapse })
            class=(class!(
                "group/sidebar relative w-0 shrink-0 text-sidebar-foreground md:sticky md:top-0 md:h-svh md:w-(--sidebar-width) md:self-start md:transition-[width] md:duration-200 md:data-[collapsible=offcanvas]:w-0 motion-reduce:transition-none",
                if variant == SidebarVariant::Floating {
                    "md:p-2 md:data-[collapsible=icon]:w-[calc(var(--sidebar-width-icon)+1rem+2px)] md:data-[collapsible=offcanvas]:px-0"
                } else {
                    "md:data-[collapsible=icon]:w-(--sidebar-width-icon)"
                },
                "md:py-2" if variant == SidebarVariant::Inset,
                attrs.remove("class"),
            ))
            (attrs)
        >
            sheet(
                open: mobile_open,
                attrs: attributes! {
                    role="navigation"
                    aria-label="Sidebar"
                    class=(class!(
                        "md:relative md:inset-auto md:flex md:overflow-visible md:bg-transparent md:opacity-100 md:backdrop-blur-none md:starting:open:opacity-100 md:transition-none md:group-data-[collapsible=offcanvas]/sidebar:invisible motion-reduce:transition-none",
                        sheet_attrs.remove("class"),
                    ))
                    (sheet_attrs)
                },
                sheet_content(
                    side: side.sheet(),
                    attrs: attributes! {
                        data-sidebar="panel"
                        class=(class!(
                            PANEL_THEME,
                            "relative [&]:w-(--sidebar-width-mobile) [&]:max-w-[calc(100vw-3rem)] [&]:gap-0 [&]:overflow-hidden [&]:p-0 md:[&]:w-full md:[&]:max-w-none md:[&]:translate-x-0 md:[&]:transition-none motion-reduce:transition-none",
                            match variant {
                                SidebarVariant::Sidebar => "md:shadow-none",
                                SidebarVariant::Floating => "md:rounded-xl md:border md:shadow-sm",
                                SidebarVariant::Inset => "md:border-0 md:bg-transparent md:shadow-none",
                            },
                        ))
                    },
                    (child)
                )
            )
        </aside>
    })
}

/// A ghost icon button that opens and closes the [`sidebar`].
///
/// Pass the handler that updates your signal as an `@click` attribute in
/// `attrs`. `open` sets `aria-expanded` and defaults to `true`. Set
/// `aria-controls` in `attrs` to the id of the panel. For a trigger that
/// works on both screen sizes, you can use different expressions and
/// handlers for desktop and mobile.
#[component]
pub async fn sidebar_trigger(
    #[into]
    #[default(true.into())]
    open: Expr<bool>,
    #[default] attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        button(
            variant: ButtonVariant::Ghost,
            size: ButtonSize::Icon,
            attrs: attributes! {
                type="button"
                data-sidebar="trigger"
                aria-label="Toggle sidebar"
                :aria-expanded=$(if open { "true" } else { "false" })
                (attrs)
            },
            icon(data: iconify_icon!("lucide:panel-left"))
        )
    })
}

/// A thin button along the inner edge of the desktop [`sidebar`] that opens
/// and closes it.
///
/// Pass the handler that updates your signal as an `@click` attribute in
/// `attrs`. `open` sets `aria-expanded` and defaults to `true`. The rail is
/// hidden on mobile.
#[component]
pub async fn sidebar_rail(
    #[into]
    #[default(true.into())]
    open: Expr<bool>,
    #[default] mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <button
            type="button"
            data-sidebar="rail"
            aria-label="Toggle sidebar"
            title="Toggle sidebar"
            :aria-expanded=$(if open { "true" } else { "false" })
            class=(class!(
                "absolute inset-y-0 z-10 hidden w-2 cursor-pointer outline-none after:absolute after:inset-y-0 after:w-px hover:after:bg-sidebar-border focus-visible:after:bg-sidebar-ring md:block group-data-[side=left]/sidebar:right-0 group-data-[side=left]/sidebar:after:right-0 group-data-[side=right]/sidebar:left-0 group-data-[side=right]/sidebar:after:left-0",
                attrs.remove("class"),
            ))
            (attrs)
        ></button>
    })
}

/// The page content next to a [`sidebar`], rendered as a `<main>`.
///
/// On desktop, it scrolls separately from the sidebar. A [`sidebar_header`]
/// that is a direct child becomes a sticky toolbar with the same height and
/// bottom border as the sidebar's header.
#[component]
pub async fn sidebar_inset(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <main
            data-sidebar="inset"
            class=(class!(
                "relative flex min-w-0 flex-1 flex-col bg-background md:min-h-0 md:overflow-y-auto [&>[data-sidebar=header]]:sticky [&>[data-sidebar=header]]:top-0 [&>[data-sidebar=header]]:z-20 [&>[data-sidebar=header]]:flex-row [&>[data-sidebar=header]]:items-center [&>[data-sidebar=header]]:justify-start [&>[data-sidebar=header]]:gap-3 [&>[data-sidebar=header]]:bg-background [&>[data-sidebar=header]]:px-4 [&>[data-sidebar=header]>[aria-orientation=vertical]]:h-4 md:in-[[data-sidebar=provider]:has([data-variant=floating])]:my-[calc(--spacing(2)+1px)] md:in-[[data-sidebar=provider]:has([data-variant=inset])]:m-2 md:in-[[data-sidebar=provider]:has([data-variant=inset][data-side=left])]:ml-0 md:in-[[data-sidebar=provider]:has([data-variant=inset][data-side=right])]:mr-0 md:in-[[data-sidebar=provider]:has([data-variant=inset])]:rounded-xl md:in-[[data-sidebar=provider]:has([data-variant=inset])]:shadow-sm md:in-[[data-sidebar=provider]:has([data-variant=inset])]:ring-1 md:in-[[data-sidebar=provider]:has([data-variant=inset])]:ring-sidebar-border",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </main>
    })
}

/// A header for the [`sidebar`] or for the page content.
///
/// Its height and bottom border stay the same when the sidebar collapses to
/// icons. Inside a [`sidebar_inset`], it lays out its children as a sticky
/// toolbar.
#[component]
pub async fn sidebar_header(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            data-sidebar="header"
            class=(class!(
                "flex h-14 shrink-0 flex-col justify-center gap-2 border-b border-border px-2",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}

/// The footer of a [`sidebar`], below the scrolling [`sidebar_content`].
#[component]
pub async fn sidebar_footer(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            data-sidebar="footer"
            class=(class!("flex shrink-0 flex-col gap-2 p-2", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// The scrolling area of a [`sidebar`], between the header and the footer.
#[component]
pub async fn sidebar_content(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            data-sidebar="content"
            class=(class!(
                "flex min-h-0 flex-1 flex-col gap-2 overflow-x-hidden overflow-y-auto overscroll-contain",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}

/// A section of the sidebar content, usually with a
/// [`sidebar_group_label`], an optional [`sidebar_group_action`], and a
/// [`sidebar_group_content`].
#[component]
pub async fn sidebar_group(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            data-sidebar="group"
            class=(class!(
                "relative flex min-w-0 flex-col gap-1 p-2",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}

/// The caption of a [`sidebar_group`], hidden when the desktop sidebar
/// collapses to icons.
#[component]
pub async fn sidebar_group_label(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            data-sidebar="group-label"
            class=(class!(
                "flex h-8 shrink-0 items-center rounded-md px-2 text-xs font-medium text-sidebar-foreground/70 md:group-data-[collapsible=icon]/sidebar:hidden",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}

// Shared button styles for group and menu actions. Each one sets its own
// position.
const ACTION: StaticClass = class!(
    "absolute flex size-6 items-center justify-center rounded-md text-sidebar-foreground/70 outline-none hover:bg-sidebar-accent/50 hover:text-sidebar-accent-foreground focus-visible:ring-2 focus-visible:ring-sidebar-ring disabled:pointer-events-none disabled:opacity-50 md:group-data-[collapsible=icon]/sidebar:hidden [&_svg]:size-4",
);

/// An icon button next to the label of a [`sidebar_group`].
///
/// Child nodes become its content, usually an icon. Give it an accessible
/// label, such as an `aria-label` in `attrs`.
#[component]
pub async fn sidebar_group_action(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <button
            type="button"
            data-sidebar="group-action"
            class=(class!(ACTION, "top-3 right-3", attrs.remove("class")))
            (attrs)
        >
            (child)
        </button>
    })
}

/// The body of a [`sidebar_group`], usually holding a [`sidebar_menu`].
#[component]
pub async fn sidebar_group_content(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            data-sidebar="group-content"
            class=(class!("w-full text-sm", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// A list of [`sidebar_menu_item`]s, rendered as a `<ul>`.
#[component]
pub async fn sidebar_menu(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <ul
            data-sidebar="menu"
            class=(class!("flex min-w-0 flex-col gap-1", attrs.remove("class")))
            (attrs)
        >
            (child)
        </ul>
    })
}

/// A row of a [`sidebar_menu`], rendered as an `<li>`.
///
/// It holds a [`sidebar_menu_button`] and optionally a
/// [`sidebar_menu_action`], a [`sidebar_menu_badge`], or a
/// [`sidebar_menu_sub`].
#[component]
pub async fn sidebar_menu_item(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <li
            data-sidebar="menu-item"
            class=(class!("group/menu-item relative", attrs.remove("class")))
            (attrs)
        >
            (child)
        </li>
    })
}

/// A text input for a sidebar header or group, hidden when the desktop
/// sidebar collapses to icons.
///
/// The `attrs` are forwarded to the `<input>`.
#[component]
pub async fn sidebar_input(#[default] mut attrs: Attributes) -> Result<impl View> {
    Ok(view! {
        input(
            attrs: attributes! {
                data-sidebar="input"
                class=(class!(
                    "bg-background md:group-data-[collapsible=icon]/sidebar:hidden",
                    attrs.remove("class"),
                ))
                (attrs)
            }
        )
    })
}

/// A thin line between sidebar sections.
#[component]
pub async fn sidebar_separator(#[default] attrs: Attributes) -> Result<impl View> {
    Ok(view! { separator(attrs: attributes! { data-sidebar="separator" (attrs) }) })
}

/// The visual style of a [`sidebar_menu_button`].
///
/// The default is `SidebarMenuButtonVariant::Default`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SidebarMenuButtonVariant {
    /// No border or fill until hovered.
    #[default]
    Default,
    /// A border, the sidebar background, and a small shadow.
    Outline,
}

impl SidebarMenuButtonVariant {
    /// The Tailwind classes for this variant.
    fn classes(self) -> StaticClass {
        match self {
            Self::Default => class!("border-transparent"),
            Self::Outline => class!("border-sidebar-border bg-sidebar shadow-xs"),
        }
    }
}

/// The size of a [`sidebar_menu_button`] or [`sidebar_menu_sub_button`].
///
/// The default is `SidebarMenuButtonSize::Md`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SidebarMenuButtonSize {
    /// A compact row with small text.
    Sm,
    /// The standard row height.
    #[default]
    Md,
    /// A tall row, for example for an item with two lines of text.
    Lg,
}

impl SidebarMenuButtonSize {
    /// The Tailwind classes for this size.
    fn classes(self) -> StaticClass {
        match self {
            Self::Sm => class!("h-7 text-xs"),
            Self::Md => class!("h-8 text-sm"),
            Self::Lg => class!("h-12 text-sm"),
        }
    }
}

/// The classes shared by every sidebar menu button.
const MENU_BUTTON: StaticClass = class!(
    "flex w-full min-w-0 items-center gap-2 overflow-hidden rounded-md border px-2 text-left outline-none transition-colors hover:bg-sidebar-accent/50 hover:text-sidebar-accent-foreground active:bg-sidebar-accent active:text-sidebar-accent-foreground focus-visible:ring-2 focus-visible:ring-sidebar-ring disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50 data-[active=true]:bg-sidebar-accent data-[active=true]:text-sidebar-accent-foreground data-[active=true]:font-medium has-[+[data-sidebar=menu-action]]:pr-8 has-[+[data-sidebar=menu-badge]]:pr-8 [&>svg]:size-4 [&>svg]:shrink-0 [&>span:last-child]:truncate md:group-data-[collapsible=icon]/sidebar:size-8 md:group-data-[collapsible=icon]/sidebar:justify-center md:group-data-[collapsible=icon]/sidebar:p-0 md:group-data-[collapsible=icon]/sidebar:[&>span:last-child]:sr-only",
);

/// Returns the full class list of a [`sidebar_menu_button`] with the given
/// `variant` and `size`.
///
/// Use it to style another element like a sidebar menu button. Set
/// `data-active="true"` for the active style. Put the label in a `<span>`
/// after the icon, so it stays available to assistive technology when the
/// sidebar collapses to icons.
#[must_use]
pub fn sidebar_menu_button_variants(
    variant: SidebarMenuButtonVariant,
    size: SidebarMenuButtonSize,
) -> Class<(StaticClass, StaticClass, StaticClass)> {
    class!(MENU_BUTTON, variant.classes(), size.classes())
}

/// The main button of a [`sidebar_menu_item`]. It is an `<a>` when `href` is
/// set, and a `<button>` otherwise.
///
/// Child nodes become its content: put an icon before a `<span>` with the
/// label. When the sidebar collapses to icons, the label is only visible to
/// assistive technology. `tooltip` sets the `title` attribute, which gives
/// the icon a native hint. `active` accepts a boolean or a runtime
/// expression, sets the active style and `aria-current="page"`, and defaults
/// to `false`. `variant` and `size` default to `Default` and `Md`. The
/// `attrs`, such as event handlers, are forwarded to the element.
#[component]
pub async fn sidebar_menu_button(
    #[default] variant: SidebarMenuButtonVariant,
    #[default] size: SidebarMenuButtonSize,
    #[into]
    #[default(false.into())]
    active: Expr<bool>,
    #[default] href: Option<&str>,
    #[default] tooltip: Option<&str>,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let attrs = attributes! {
        data-sidebar="menu-button"
        :data-active=$(if active { "true" } else { "false" })
        :aria-current=$(active.then_some("page"))
        title=(tooltip)
        class=(class!(
            sidebar_menu_button_variants(variant, size),
            attrs.remove("class"),
        ))
        (attrs)
    };

    Ok(view! {
        if let Some(href) = href {
            <a href=(href) (attrs)>(child)</a>
        } else {
            <button type="button" (attrs)>(child)</button>
        }
    })
}

/// A small icon button at the right end of a [`sidebar_menu_item`], separate
/// from its [`sidebar_menu_button`].
///
/// Child nodes become its content, usually an icon. Give it an accessible
/// label, such as an `aria-label` in `attrs`. With `show_on_hover`, it is
/// only visible on desktop while the item is hovered or has focus.
#[component]
pub async fn sidebar_menu_action(
    #[default] show_on_hover: bool,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <button
            type="button"
            data-sidebar="menu-action"
            class=(class!(
                ACTION,
                "top-1 right-1",
                "md:opacity-0 md:group-hover/menu-item:opacity-100 md:group-focus-within/menu-item:opacity-100" if show_on_hover,
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </button>
    })
}

/// A count or status at the right end of a [`sidebar_menu_item`], hidden when
/// the desktop sidebar collapses to icons.
#[component]
pub async fn sidebar_menu_badge(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span
            data-sidebar="menu-badge"
            class=(class!(
                "pointer-events-none absolute top-1 right-1 flex h-6 min-w-6 items-center justify-center rounded-md px-1 text-xs tabular-nums text-sidebar-foreground/70 md:group-data-[collapsible=icon]/sidebar:hidden",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </span>
    })
}

/// A loading placeholder with the shape of a menu row.
///
/// With `show_icon`, it also shows a placeholder for the icon. It is hidden
/// from assistive technology.
#[component]
pub async fn sidebar_menu_skeleton(
    #[default] show_icon: bool,
    #[default] mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <div
            data-sidebar="menu-skeleton"
            aria-hidden="true"
            class=(class!(
                "flex h-8 items-center gap-2 rounded-md px-2",
                attrs.remove("class"),
            ))
            (attrs)
        >
            if show_icon {
                skeleton(attrs: attributes! { class="size-4 shrink-0" })
            }
            skeleton(
                attrs: attributes! {
                    class="h-4 w-3/4 md:group-data-[collapsible=icon]/sidebar:hidden"
                }
            )
        </div>
    })
}

/// A nested menu inside a [`sidebar_menu_item`], rendered as a `<ul>`.
///
/// It is hidden when the desktop sidebar collapses to icons.
#[component]
pub async fn sidebar_menu_sub(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <ul
            data-sidebar="menu-sub"
            class=(class!(
                "mx-3.5 flex min-w-0 flex-col gap-1 border-l border-sidebar-border px-2.5 py-1 md:group-data-[collapsible=icon]/sidebar:hidden",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </ul>
    })
}

/// A row of a [`sidebar_menu_sub`], rendered as an `<li>`.
#[component]
pub async fn sidebar_menu_sub_item(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <li
            data-sidebar="menu-sub-item"
            class=(class!("relative", attrs.remove("class")))
            (attrs)
        >
            (child)
        </li>
    })
}

/// A link in a [`sidebar_menu_sub_item`], rendered as an `<a>`.
///
/// Pass the `href` in `attrs`. `active` accepts a boolean or a runtime
/// expression, sets the active style and `aria-current="page"`, and defaults
/// to `false`. `size` defaults to `Md`.
#[component]
pub async fn sidebar_menu_sub_button(
    #[into]
    #[default(false.into())]
    active: Expr<bool>,
    #[default] size: SidebarMenuButtonSize,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <a
            data-sidebar="menu-sub-button"
            :aria-current=$(active.then_some("page"))
            class=(class!(
                "flex min-w-0 items-center gap-2 rounded-md px-2 text-sidebar-foreground/70 outline-none hover:bg-sidebar-accent/50 hover:text-sidebar-accent-foreground focus-visible:ring-2 focus-visible:ring-sidebar-ring aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-accent-foreground aria-disabled:pointer-events-none aria-disabled:opacity-50 [&>svg]:size-4 [&>svg]:shrink-0 [&>span:last-child]:truncate",
                size.classes(),
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </a>
    })
}
