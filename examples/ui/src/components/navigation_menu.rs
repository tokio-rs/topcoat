use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    runtime::Expr,
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

/// A row of site navigation links, some of which open a panel of further links.
///
/// Panels open without JavaScript. A panel opens while the pointer rests on its
/// item or while keyboard focus is inside it. On touch screens, tapping the trigger
/// opens the panel and tapping elsewhere closes it. Pressing Escape does not close a
/// panel. Items use normal Tab navigation and do not implement arrow-key navigation.
///
/// `attrs` are forwarded to the `<nav>`, with extra classes added to its classes. Give
/// it an `aria-label` when the page has more than one navigation region.
///
/// ```ignore
/// view! {
///     navigation_menu(
///         navigation_menu_list(
///             navigation_menu_item(
///                 navigation_menu_trigger("Guides")
///                 navigation_menu_content(
///                     attrs: attributes! { class="w-80" },
///                     navigation_menu_link(
///                         attrs: attributes! { href="/guides/start" },
///                         <span class="font-medium">"Getting started"</span>
///                         <span class="text-muted-foreground">"Install and run an app."</span>
///                     )
///                 )
///             )
///             navigation_menu_item(
///                 navigation_menu_link(
///                     variant: NavigationMenuLinkVariant::Trigger,
///                     active: true,
///                     attrs: attributes! { href="/blog" },
///                     "Blog"
///                 )
///             )
///         )
///     )
/// }
/// ```
#[component]
pub async fn navigation_menu(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <nav
            class=(class!("relative flex items-center", attrs.remove("class")))
            (attrs)
        >
            (child)
        </nav>
    })
}

/// The list of items in a [`navigation_menu`].
#[component]
pub async fn navigation_menu_list(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <ul
            class=(class!("flex list-none items-center gap-1", attrs.remove("class")))
            (attrs)
        >
            (child)
        </ul>
    })
}

/// One entry of a [`navigation_menu_list`].
///
/// Pass either a top-level [`navigation_menu_link`], or a [`navigation_menu_trigger`]
/// followed by its [`navigation_menu_content`]. `attrs` are forwarded to the `<li>`.
#[component]
pub async fn navigation_menu_item(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    // The negative tab index lets a tap on the trigger focus the item in browsers
    // that do not focus buttons on click, so the panel opens on touch screens.
    Ok(view! {
        <li
            tabindex="-1"
            class=(class!(
                "group/navigation-item relative outline-none",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </li>
    })
}

/// Classes for top-level triggers and links, with hover, focus, and current-page states.
const TRIGGER: StaticClass = class!(
    "inline-flex h-9 shrink-0 items-center justify-center gap-1 rounded-md px-3 text-sm \
     font-medium whitespace-nowrap text-muted-foreground transition-colors outline-none \
     hover:bg-foreground/5 hover:text-foreground \
     focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
     focus-visible:ring-offset-background \
     aria-[current=page]:text-foreground disabled:pointer-events-none disabled:opacity-50",
);

/// A button that opens the [`navigation_menu_content`] after it.
///
/// Pass its label as children. A chevron turns while the panel is open. `attrs` are
/// forwarded to the `<button>`, with extra classes added to its classes.
#[component]
pub async fn navigation_menu_trigger(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <button
            type="button"
            class=(class!(
                TRIGGER,
                "group-hover/navigation-item:text-foreground \
                 group-has-[:focus-visible]/navigation-item:text-foreground \
                 pointer-coarse:group-focus-within/navigation-item:text-foreground",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
            icon(
                data: iconify_icon!("lucide:chevron-down"),
                attrs: attributes! {
                    class="size-3.5 shrink-0 opacity-60 transition-transform duration-200 \
                        group-hover/navigation-item:rotate-180 \
                        group-has-[:focus-visible]/navigation-item:rotate-180 \
                        pointer-coarse:group-focus-within/navigation-item:rotate-180"
                }
            )
        </button>
    })
}

/// Classes that place the panel below its item and reveal it while the item is open.
///
/// The top padding bridges the gap to the trigger, so the pointer can move onto the
/// panel without closing it.
const LAYER: StaticClass = class!(
    "invisible absolute top-full left-0 z-50 translate-y-1 pt-2 opacity-0 \
     [transition:opacity_150ms_ease-out,translate_150ms_ease-out,visibility_150ms_allow-discrete] \
     group-hover/navigation-item:visible group-hover/navigation-item:translate-y-0 \
     group-hover/navigation-item:opacity-100 \
     group-has-[:focus-visible]/navigation-item:visible \
     group-has-[:focus-visible]/navigation-item:translate-y-0 \
     group-has-[:focus-visible]/navigation-item:opacity-100 \
     pointer-coarse:group-focus-within/navigation-item:visible \
     pointer-coarse:group-focus-within/navigation-item:translate-y-0 \
     pointer-coarse:group-focus-within/navigation-item:opacity-100",
);

/// The panel a [`navigation_menu_trigger`] opens, holding further links.
///
/// The panel drops below its item, aligned to the item's left edge. `attrs` are
/// forwarded to the panel `<div>`. Set its width and layout through extra classes,
/// for example a grid of [`navigation_menu_link`]s.
#[component]
pub async fn navigation_menu_content(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class=(LAYER)>
            <div
                class=(class!(
                    "rounded-xl border border-border bg-popover p-1.5 text-popover-foreground \
                     shadow-sm",
                    attrs.remove("class"),
                ))
                (attrs)
            >
                (child)
            </div>
        </div>
    })
}

/// The visual style of a [`navigation_menu_link`].
///
/// [`Default`] is `NavigationMenuLinkVariant::Panel`, used when no variant is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum NavigationMenuLinkVariant {
    /// A row inside a [`navigation_menu_content`] panel that stacks its children.
    #[default]
    Panel,
    /// A top-level link styled like a [`navigation_menu_trigger`].
    Trigger,
}

impl NavigationMenuLinkVariant {
    /// Classes for the link variant and its interaction states.
    fn classes(self) -> StaticClass {
        match self {
            Self::Panel => class!(
                "flex flex-col gap-1 rounded-lg px-3 py-2.5 text-sm leading-snug \
                 outline-none transition-colors hover:bg-foreground/5 \
                 focus-visible:bg-foreground/5 focus-visible:ring-2 focus-visible:ring-ring \
                 aria-[current=page]:bg-foreground/5 [&>svg]:size-4 [&>svg]:shrink-0",
            ),
            Self::Trigger => TRIGGER,
        }
    }
}

/// Classes for styling another element as a navigation menu link.
///
/// Set `aria-current="page"` for current-page styling.
#[must_use]
pub fn navigation_menu_link_variants(variant: NavigationMenuLinkVariant) -> StaticClass {
    variant.classes()
}

/// A navigation link, rendered as an `<a>`.
///
/// Pass the destination as `href` in `attrs`. `active` accepts a boolean or runtime
/// expression and marks the link with `aria-current="page"`.
#[component]
pub async fn navigation_menu_link(
    /// How the link is styled.
    #[default]
    variant: NavigationMenuLinkVariant,
    /// Whether the link points to the current page.
    #[into]
    #[default(false.into())]
    active: Expr<bool>,
    /// Extra attributes for the `<a>` element.
    #[default]
    mut attrs: Attributes,
    /// The link's content.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <a
            :aria-current=$(active.then_some("page"))
            class=(class!(navigation_menu_link_variants(variant), attrs.remove("class")))
            (attrs)
        >
            (child)
        </a>
    })
}
