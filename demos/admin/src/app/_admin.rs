//! The `_admin` group shares a layout without adding a URL segment.

pub mod _dashboard;
mod customers;
mod orders;
mod products;

use topcoat::{
    Result,
    context::{Cx, try_context},
    icon::{IconData, icon, iconify::iconify_icon},
    l10n::{Translation, fluent::messages},
    router::{HrefTarget, Slot, href, layout, request::uri},
    runtime::{Event, Signal, link, link_attrs, prefetch_mode, signal},
    view::{View, attributes, component, element_id, view},
};

use super::{_common::role_label, language::language_menu, logout};
use crate::{
    components::{
        avatar::{AvatarSize, avatar, avatar_fallback},
        breadcrumb::{
            breadcrumb, breadcrumb_item, breadcrumb_link, breadcrumb_list, breadcrumb_page,
            breadcrumb_separator,
        },
        button::{ButtonSize, ButtonVariant, button},
        dropdown_menu::{
            dropdown_menu, dropdown_menu_content, dropdown_menu_item, dropdown_menu_label,
            dropdown_menu_separator, dropdown_menu_trigger,
        },
        floating::{FloatingAlign, FloatingSide},
        separator::{SeparatorOrientation, separator},
        sidebar::{
            SidebarCollapsible, SidebarMenuButtonSize, SidebarMenuButtonVariant, sidebar,
            sidebar_content, sidebar_footer, sidebar_group, sidebar_group_content,
            sidebar_group_label, sidebar_header, sidebar_inset, sidebar_menu,
            sidebar_menu_button_variants, sidebar_menu_item, sidebar_provider, sidebar_trigger,
        },
    },
    features::{
        auth::require_user,
        theme::{Theme, theme},
        user::User,
    },
};

messages!("{mod}/l10n/{locale}.ftl");

struct NavItem {
    label: fn() -> Translation<'static>,
    icon: IconData,
    target: &'static (dyn HrefTarget + Sync),
}

static NAV: [NavItem; 4] = [
    NavItem {
        label: || t!(nav_dashboard),
        icon: iconify_icon!("lucide:layout-dashboard"),
        target: &_dashboard::page,
    },
    NavItem {
        label: || t!(nav_orders),
        icon: iconify_icon!("lucide:shopping-cart"),
        target: &orders::page,
    },
    NavItem {
        label: || t!(nav_products),
        icon: iconify_icon!("lucide:package"),
        target: &products::page,
    },
    NavItem {
        label: || t!(nav_customers),
        icon: iconify_icon!("lucide:users"),
        target: &customers::page,
    },
];

/// A breadcrumb label for a detail page. Add it to the request context in a layer.
pub struct Crumb(pub String);

fn in_section(cx: &Cx, path: &str) -> bool {
    let current = uri(cx).path();
    current == path
        || (path != "/"
            && current
                .strip_prefix(path)
                .is_some_and(|rest| rest.starts_with('/')))
}

#[layout]
async fn shell(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let user = require_user(cx).await?;

    let open = signal(cx, || true);
    let mobile_open = signal(cx, || false);
    let sidebar_id = element_id(cx);

    Ok(view! {
        sidebar_provider(
            app_sidebar(
                id: &sidebar_id,
                user: user,
                open: &open,
                mobile_open: &mobile_open
            )
            sidebar_inset(
                sidebar_header(
                    sidebar_trigger(
                        open: $(open.get()),
                        attrs: attributes! {
                            class="max-md:hidden"
                            aria-controls=(&sidebar_id)
                            @click=$(|_e: Event| open.toggle())
                        }
                    )
                    sidebar_trigger(
                        open: $(mobile_open.get()),
                        attrs: attributes! {
                            class="md:hidden"
                            aria-controls=(&sidebar_id)
                            @click=$(|_e: Event| mobile_open.toggle())
                        }
                    )
                    separator(orientation: SeparatorOrientation::Vertical)
                    page_breadcrumb()
                    theme_toggle()
                    language_menu()
                )
                <div class="mx-auto flex w-full max-w-6xl flex-1 flex-col gap-6 p-6">
                    (slot)
                </div>
            )
        )
    })
}

#[component]
async fn app_sidebar(
    cx: &Cx,
    id: &str,
    user: &User,
    open: &Signal<bool>,
    mobile_open: &Signal<bool>,
) -> Result<impl View> {
    Ok(view! {
        sidebar(
            open: $(open.get()),
            mobile_open: $(mobile_open.get()),
            collapsible: SidebarCollapsible::Icon,
            sheet_attrs: attributes! {
                id=(id)
                aria-label=(t!(navigation))
                @keydown=$(|e: Event| {
                    if e.key == "Escape" {
                        mobile_open.set(false);
                    }
                })
                @click=$(|e: Event| {
                    // Backdrop clicks target the sheet; clicks inside it target its children.
                    if e.target.id == id {
                        mobile_open.set(false);
                    }
                })
            },
            sidebar_header(
                <div class="flex items-center gap-1">
                    <div
                        class=(sidebar_menu_button_variants(
                            SidebarMenuButtonVariant::Default,
                            SidebarMenuButtonSize::Lg,
                        ))
                    >
                        icon(data: iconify_icon!("lucide:store"))
                        <span class="font-semibold">"Backroom"</span>
                    </div>
                    button(
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Icon,
                        attrs: attributes! {
                            type="button"
                            class="md:hidden"
                            aria-label=(t!(close_sidebar))
                            @click=$(|_e: Event| mobile_open.set(false))
                        },
                        icon(data: iconify_icon!("lucide:x"))
                    )
                </div>
            )
            sidebar_content(
                sidebar_group(
                    sidebar_group_label((t!(store)))
                    sidebar_group_content(
                        sidebar_menu(
                            for item in &NAV {
                                let href = href!(item.target);
                                let current = in_section(cx, &href.resolve(cx));
                                sidebar_menu_item(
                                    link(
                                        href: href,
                                        attrs: attributes! {
                                            data-sidebar="menu-button"
                                            data-active=(if current { "true" } else { "false" })
                                            aria-current=(current.then_some("page"))
                                            title=((item.label)())
                                            class=(sidebar_menu_button_variants(
                                                SidebarMenuButtonVariant::Default,
                                                SidebarMenuButtonSize::Md,
                                            ))
                                            @click=$(|_e: Event| mobile_open.set(false))
                                        },
                                        icon(data: item.icon.clone())
                                        <span>((item.label)())</span>
                                    )
                                )
                            }
                        )
                    )
                )
            )
            sidebar_footer(sidebar_menu(sidebar_menu_item(user_menu(user: user))))
        )
    })
}

#[component]
async fn user_menu(cx: &Cx, user: &User) -> Result<impl View> {
    let menu = element_id(cx);

    Ok(view! {
        dropdown_menu(
            dropdown_menu_trigger(
                target: &menu,
                attrs: attributes! {
                    title=(&user.name)
                    class=(sidebar_menu_button_variants(
                        SidebarMenuButtonVariant::Default,
                        SidebarMenuButtonSize::Lg,
                    ))
                },
                avatar(size: AvatarSize::Sm, avatar_fallback((user.initials())))
                // The sidebar hides its last `<span>` when collapsed, so keep the avatar outside it.
                <span class="flex min-w-0 flex-1 items-center gap-2">
                    <span class="flex min-w-0 flex-col">
                        <span class="truncate font-medium">(&user.name)</span>
                        <span class="truncate text-xs text-muted-foreground">
                            (role_label(user.role))
                        </span>
                    </span>
                    icon(
                        data: iconify_icon!("lucide:chevrons-up-down"),
                        attrs: attributes! { class="ml-auto" }
                    )
                </span>
            )
            dropdown_menu_content(
                id: &menu,
                side: FloatingSide::Right,
                align: FloatingAlign::End,
                dropdown_menu_label(
                    <span class="block text-sm text-foreground">(&user.name)</span>
                    (&user.email)
                )
                dropdown_menu_separator()
                <form method="post" action=(href!(logout::submit))>
                    dropdown_menu_item(
                        attrs: attributes! { type="submit" },
                        icon(data: iconify_icon!("lucide:log-out"))
                        (t!(sign_out))
                    )
                </form>
            )
        )
    })
}

#[component]
async fn page_breadcrumb(cx: &Cx) -> Result<impl View> {
    let section = NAV
        .iter()
        .map(|item| (item, href!(item.target)))
        .find(|(_, href)| in_section(cx, &href.resolve(cx)));
    let crumb = try_context::<Crumb>(cx);

    Ok(view! {
        breadcrumb(
            breadcrumb_list(
                breadcrumb_item("Backroom")
                if let Some((item, href)) = section {
                    breadcrumb_separator()
                    breadcrumb_item(
                        if href.is_current(cx) {
                            breadcrumb_page(((item.label)()))
                        } else {
                            breadcrumb_link(
                                attrs: attributes! { (link_attrs(cx, href, prefetch_mode(cx))) },
                                ((item.label)())
                            )
                        }
                    )
                    if let Some(Crumb(label)) = crumb {
                        breadcrumb_separator()
                        breadcrumb_item(breadcrumb_page((label)))
                    }
                }
            )
        )
    })
}

#[component]
async fn theme_toggle(cx: &Cx) -> Result<impl View> {
    let dark = theme(cx) == Theme::Dark;
    let label = if dark {
        t!(light_theme)
    } else {
        t!(dark_theme)
    };
    let return_to = uri(cx).path_and_query().map_or("/", |path| path.as_str());

    Ok(view! {
        <form method="post" action=(href!(super::theme::toggle)) class="ml-auto">
            <input type="hidden" name="return_to" value=(return_to)>
            button(
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Icon,
                attrs: attributes! { type="submit" aria-label=(label.clone()) title=(label) },
                if dark {
                    icon(data: iconify_icon!("lucide:sun"))
                } else {
                    icon(data: iconify_icon!("lucide:moon"))
                }
            )
        </form>
    })
}
