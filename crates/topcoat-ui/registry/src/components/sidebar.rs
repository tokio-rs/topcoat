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

/// The edge of the page occupied by a sidebar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SidebarSide {
    #[default]
    Left,
    Right,
}

impl SidebarSide {
    fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    fn sheet(self) -> SheetSide {
        match self {
            Self::Left => SheetSide::Left,
            Self::Right => SheetSide::Right,
        }
    }
}

/// The desktop sidebar's surface and its relationship to the page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SidebarVariant {
    /// A panel separated from the page by a border.
    #[default]
    Sidebar,
    /// A rounded panel with space around it.
    Floating,
    /// A sidebar beside a rounded [`sidebar_inset`] content surface.
    Inset,
}

impl SidebarVariant {
    fn name(self) -> &'static str {
        match self {
            Self::Sidebar => "sidebar",
            Self::Floating => "floating",
            Self::Inset => "inset",
        }
    }
}

/// How the desktop sidebar behaves when its `open` expression is false.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SidebarCollapsible {
    /// Hide the panel and release its space to the page.
    #[default]
    Offcanvas,
    /// Keep a narrow strip of menu icons visible.
    Icon,
    /// Keep the desktop panel expanded.
    None,
}

impl SidebarCollapsible {
    fn name(self) -> &'static str {
        match self {
            Self::Offcanvas => "offcanvas",
            Self::Icon => "icon",
            Self::None => "none",
        }
    }
}

/// The flex layout surrounding a sidebar and the page content.
///
/// Set `--sidebar-width`, `--sidebar-width-mobile` and `--sidebar-width-icon`
/// through `attrs` to customize the widths. Place a right sidebar after the
/// page content. State belongs to the caller, which passes expressions to
/// [`sidebar`] and event handlers to [`sidebar_trigger`] and [`sidebar_rail`].
#[component]
pub async fn sidebar_provider(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            data-sidebar="provider"
            class=(class!(
                "flex min-h-svh w-full [--sidebar-width:16rem] [--sidebar-width-mobile:18rem] [--sidebar-width-icon:3rem] has-[[data-variant=inset]]:bg-sidebar",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}

// Ordinary components nested in the panel inherit its palette too. The
// scoped aliases keep buttons, inputs and captions independent of the page.
const PANEL_THEME: StaticClass = class!(
    "[--background:var(--sidebar)] [--foreground:var(--sidebar-foreground)] \
     [--card:var(--sidebar)] [--card-foreground:var(--sidebar-foreground)] \
     [--primary:var(--sidebar-primary)] \
     [--primary-foreground:var(--sidebar-primary-foreground)] \
     [--border:var(--sidebar-border)] [--ring:var(--sidebar-ring)] \
     [--muted-foreground:color-mix(in_oklab,var(--sidebar-foreground)_70%,transparent)]",
);

/// A desktop panel that becomes a sheet over the page below `md` (48rem).
///
/// The `--sidebar-*` theme tokens control its colors independently of cards,
/// sheets and page content. Nested controls inherit the sidebar palette.
///
/// `open` controls desktop expansion. `mobile_open` independently controls
/// the mobile sheet, so it can start closed while desktop starts expanded.
/// Pass runtime expressions reading the caller's signals to make both live.
/// Header, content and footer children are rendered once and shared between
/// viewport sizes, preserving their inputs and signal identities.
///
/// `attrs` apply to the outer wrapper. `sheet_attrs` apply to the sheet and
/// accept a label, an ID, and event handlers for Escape or backdrop dismissal.
/// Include a close button in the mobile header. Like [`sheet`], this is a
/// visual overlay; focus trapping requires additional application scripting.
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
                "group/sidebar relative w-0 shrink-0 text-sidebar-foreground md:sticky md:top-0 md:h-svh md:w-(--sidebar-width) md:self-start md:transition-[width] md:duration-200 md:data-[collapsible=offcanvas]:w-0 md:data-[collapsible=icon]:w-(--sidebar-width-icon) motion-reduce:transition-none",
                "md:p-2" if variant == SidebarVariant::Floating,
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
                        "md:relative md:inset-auto md:flex md:h-full md:w-full md:overflow-visible md:bg-transparent md:opacity-100 md:backdrop-blur-none md:open:flex md:starting:open:opacity-100 md:transition-none md:group-data-[collapsible=offcanvas]/sidebar:invisible motion-reduce:transition-none",
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
                                SidebarVariant::Inset => "md:border-0 md:shadow-none",
                            },
                        ))
                    },
                    (child)
                )
            )
        </aside>
    })
}

/// A sidebar toggle button. Pass its signal update as an `@click` attribute.
///
/// `open` controls the accessible expanded state. Set `aria-controls` in
/// `attrs` to the controlled panel's ID. Responsive triggers can use separate
/// desktop and mobile expressions and handlers.
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

/// A desktop edge button that toggles the sidebar on click.
///
/// Pass its signal update as an `@click` attribute.
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

/// The page content beside a sidebar.
///
/// An immediate [`sidebar_header`] child becomes a sticky toolbar. Its height
/// and bottom border match the sidebar header without extra classes.
#[component]
pub async fn sidebar_inset(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <main
            data-sidebar="inset"
            class=(class!(
                "relative flex min-w-0 flex-1 flex-col bg-background [&>[data-sidebar=header]]:sticky [&>[data-sidebar=header]]:top-0 [&>[data-sidebar=header]]:z-20 [&>[data-sidebar=header]]:flex-row [&>[data-sidebar=header]]:items-center [&>[data-sidebar=header]]:justify-start [&>[data-sidebar=header]]:gap-3 [&>[data-sidebar=header]]:bg-background [&>[data-sidebar=header]]:px-4 [&>[data-sidebar=header]>[aria-orientation=vertical]]:h-4 md:in-[[data-sidebar=provider]:has([data-variant=inset])]:m-2 md:in-[[data-sidebar=provider]:has([data-variant=inset][data-side=left])]:ml-0 md:in-[[data-sidebar=provider]:has([data-variant=inset][data-side=right])]:mr-0 md:in-[[data-sidebar=provider]:has([data-variant=inset])]:rounded-xl md:in-[[data-sidebar=provider]:has([data-variant=inset])]:shadow-sm",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </main>
    })
}

/// A header for the sidebar or its page content.
///
/// Its height and divider stay fixed when menu buttons collapse to icons.
/// Inside [`sidebar_inset`], it lays out its children as a sticky toolbar.
#[component]
pub async fn sidebar_header(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            data-sidebar="header"
class=(class!("flex h-14 shrink-0 flex-col justify-center gap-2 border-b border-border px-2", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// The fixed footer below the sidebar's scrolling content.
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

/// The scrollable region between the header and footer.
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

/// A section containing a label, an optional action and related menu items.
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

/// A group's caption, hidden when the desktop sidebar collapses to icons.
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

// Shared button styles for group and menu actions. Each sets its own position.
const ACTION: StaticClass = class!(
    "absolute flex size-6 items-center justify-center rounded-md text-sidebar-foreground/70 outline-none hover:bg-sidebar-accent/50 hover:text-sidebar-accent-foreground focus-visible:ring-2 focus-visible:ring-sidebar-ring disabled:pointer-events-none disabled:opacity-50 md:group-data-[collapsible=icon]/sidebar:hidden [&_svg]:size-4",
);

/// An icon button beside the group label. Give it an accessible label.
#[component]
pub async fn sidebar_group_action(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <button
            type="button"
            data-sidebar="group-action"
            class=(class!(
                ACTION,
                "top-3 right-3",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </button>
    })
}

/// The body of a sidebar group.
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

/// A list of sidebar menu items.
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

/// One menu row containing a button or link and optional actions or submenus.
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

/// A text input styled for a sidebar header or group.
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

/// A rule between sidebar sections.
#[component]
pub async fn sidebar_separator(#[default] attrs: Attributes) -> Result<impl View> {
    Ok(view! {
        separator(
            attrs: attributes! {
                data-sidebar="separator"
                (attrs)
            }
        )
    })
}

/// The visual style of a sidebar menu button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SidebarMenuButtonVariant {
    #[default]
    Default,
    Outline,
}

impl SidebarMenuButtonVariant {
    fn classes(self) -> StaticClass {
        match self {
            Self::Default => class!("border-transparent"),
            Self::Outline => class!("border-sidebar-border bg-sidebar shadow-xs"),
        }
    }
}

/// The size of a sidebar menu button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SidebarMenuButtonSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl SidebarMenuButtonSize {
    fn classes(self) -> StaticClass {
        match self {
            Self::Sm => class!("h-7 text-xs"),
            Self::Md => class!("h-8 text-sm"),
            Self::Lg => class!("h-12 text-sm"),
        }
    }
}

const MENU_BUTTON: StaticClass = class!(
    "peer/menu-button flex w-full min-w-0 items-center gap-2 overflow-hidden rounded-md border px-2 text-left outline-none transition-colors hover:bg-sidebar-accent/50 hover:text-sidebar-accent-foreground active:bg-sidebar-accent active:text-sidebar-accent-foreground focus-visible:ring-2 focus-visible:ring-sidebar-ring disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50 data-[active=true]:bg-sidebar-accent data-[active=true]:text-sidebar-accent-foreground data-[active=true]:font-medium has-[+[data-sidebar=menu-action]]:pr-8 has-[+[data-sidebar=menu-badge]]:pr-8 [&>svg]:size-4 [&>svg]:shrink-0 [&>span:last-child]:truncate md:group-data-[collapsible=icon]/sidebar:size-8 md:group-data-[collapsible=icon]/sidebar:justify-center md:group-data-[collapsible=icon]/sidebar:p-0 md:group-data-[collapsible=icon]/sidebar:[&>span:last-child]:sr-only",
);

/// Classes for styling another element as a sidebar menu button.
///
/// Set `data-active="true"` for active styling. Put the label in a `<span>`
/// after the icon so it remains accessible when the sidebar collapses.
#[must_use]
pub fn sidebar_menu_button_variants(
    variant: SidebarMenuButtonVariant,
    size: SidebarMenuButtonSize,
) -> Class<(StaticClass, StaticClass, StaticClass)> {
    class!(MENU_BUTTON, variant.classes(), size.classes())
}

/// A menu action, or a navigation link when `href` is supplied.
///
/// Put an icon before a `<span>` containing the label. `tooltip` supplies a
/// native title hint for the icon-only state. `active` accepts a boolean or
/// runtime expression. Event handlers and other attributes are forwarded.
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

/// An independent action beside a menu button. Give it an accessible label.
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

/// A count or status beside a menu button.
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

/// A placeholder matching a menu row, optionally including an icon.
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

/// A nested menu, hidden when the sidebar collapses to icons.
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

/// One row of a nested sidebar menu.
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

/// A nested menu link. Pass its destination through the `href` attribute.
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
