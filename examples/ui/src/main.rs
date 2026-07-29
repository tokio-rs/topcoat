mod components;

use components::accordion::{accordion, accordion_content, accordion_item, accordion_trigger};
use components::alert::{AlertVariant, alert, alert_description, alert_title};
use components::alert_dialog::alert_dialog;
use components::avatar::{AvatarSize, avatar, avatar_fallback, avatar_image};
use components::badge::{BadgeVariant, badge, badge_variants};
use components::breadcrumb::{
    breadcrumb, breadcrumb_ellipsis, breadcrumb_item, breadcrumb_link, breadcrumb_list,
    breadcrumb_page, breadcrumb_separator,
};
use components::button::{ButtonSize, ButtonVariant, button, button_variants};
use components::card::{
    card, card_content, card_description, card_footer, card_header, card_title,
};
use components::checkbox::checkbox;
use components::dialog::{
    dialog, dialog_content, dialog_description, dialog_footer, dialog_header, dialog_title,
};
use components::dropdown_menu::{
    dropdown_menu, dropdown_menu_content, dropdown_menu_item, dropdown_menu_label,
    dropdown_menu_separator, dropdown_menu_sub, dropdown_menu_sub_content,
    dropdown_menu_sub_trigger, dropdown_menu_trigger,
};
use components::hover_card::{hover_card, hover_card_content};
use components::input::input;
use components::kbd::{kbd, kbd_group};
use components::label::label;
use components::pagination::{
    pagination, pagination_content, pagination_ellipsis, pagination_item, pagination_link,
    pagination_next, pagination_previous,
};
use components::progress::progress;
use components::radio_group::{radio_group, radio_group_item};
use components::select::select;
use components::separator::{SeparatorOrientation, separator};
use components::sheet::{SheetSide, sheet, sheet_content};
use components::skeleton::skeleton;
use components::spinner::spinner;
use components::switch::switch;
use components::table::{
    table, table_body, table_caption, table_cell, table_footer, table_head, table_header, table_row,
};
use components::tabs::{tabs, tabs_content, tabs_list, tabs_trigger};
use components::textarea::textarea;
use components::toggle::{ToggleKind, ToggleSize, toggle, toggle_group};
use components::tooltip::{tooltip, tooltip_content};
use topcoat::{
    Result,
    asset::{Asset, AssetBundle, RouterBuilderAssetExt, asset},
    context::Cx,
    font::fontsource::fontsource_font,
    icon::{icon, iconify::iconify_icon},
    router::{Router, RouterBuilderDiscoverExt, page, query_params},
    tailwind,
    view::{View, attributes, class, component, view},
};

/// A stand-in portrait for the team roster's first member, served from the
/// example's own asset bundle.
const PORTRAIT: Asset = asset!("./portrait.svg");

#[tokio::main]
async fn main() {
    let router = Router::builder()
        .assets(AssetBundle::load().unwrap())
        .discover()
        .build();

    topcoat::start(router).await.unwrap();
}

/// What the page keeps in its URL: the deployments table's page, and whether
/// the rename dialog is open. A query string that makes no sense reloads the
/// page without one.
#[query_params(error = redirect("?"))]
struct HomeQuery {
    page: Option<usize>,
    tab: Option<String>,
    dialog: Option<String>,
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    // Every state the page can be in is in its URL: which page of the table
    // is showing, which tab is open, and which of the overlays is up. The
    // links in them navigate, and this reads back what they set.
    let query = query_params::<HomeQuery>(cx)?;
    let page = query.page.unwrap_or(1).clamp(1, PAGES);
    let tab = query.tab.as_deref().unwrap_or(TABS[0].0);
    let overlay = query.dialog.as_deref();

    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Topcoat UI"</title>
                topcoat::dev::script()
                topcoat::font::link(font: fontsource_font!(GEIST, host: Asset))
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
            </head>
            // The body's background, text color, and font come from the
            // theme's base layer in styles.css; nothing to set up here.
            <body>
                <main class="mx-auto max-w-6xl px-6 py-16">
                    <header class="max-w-2xl">
                        <h1 class="text-4xl font-bold tracking-tight">
                            "Build your component library"
                        </h1>
                        <p class="mt-3 text-muted-foreground">
                            "Accessible, themeable components vendored into your \
                             project with "
                            <code class="text-foreground">"topcoat ui add"</code>
                            ". Yours to restyle, rewrite, and ship."
                        </p>
                        <div class="mt-6 flex flex-wrap items-center gap-3">
                            button(
                                size: ButtonSize::Lg,
                                "Get started"
                                icon(data: iconify_icon!("feather:arrow-right"))
                            )
                            // Anything can borrow a button's looks:
                            // `button_variants` returns the class string for a
                            // variant and size.
                            <a
                                href="https://github.com/tokio-rs/topcoat"
                                class=(button_variants(
                                    ButtonVariant::Outline,
                                    ButtonSize::Lg,
                                ))
                            >
                                "View on GitHub"
                            </a>
                        </div>
                    </header>

                    // A masonry of small, self-contained demos, each built
                    // from the installed components.
                    <div class="mt-14 columns-1 gap-4 sm:columns-2 xl:columns-3">
                        demo(team_card())
                        demo(buttons_card())
                        demo(sign_in_card())
                        demo(alerts_card())
                        demo(upgrade_card())
                        demo(plan_card())
                        demo(deploy_card())
                        demo(overview_card(tab: tab))
                        demo(deployments_card(page: page))
                        demo(delete_card())
                        demo(toolbar_card())
                        demo(status_card())
                        demo(branches_card())
                        demo(docs_card())
                        demo(share_card())
                        demo(settings_card())
                        demo(project_card())
                        demo(loading_card())
                        demo(feedback_card())
                        demo(faq_card())
                        demo(notifications_card())
                        demo(rename_card())
                        demo(hints_card())
                        demo(links_card())
                    </div>
                </main>

                // The overlays cover the page, so they stand outside the
                // masonry rather than in the cells that open them.
                rename_dialog(open: overlay == Some("rename"))
                delete_dialog(open: overlay == Some("delete"))
                filters_sheet(open: overlay == Some("filters"))
            </body>
        </html>
    }
}

/// A masonry cell: keeps a demo from splitting across columns.
#[component]
async fn demo(child: View) -> Result {
    view! { <div class="mb-4 break-inside-avoid">(child)</div> }
}

/// A team roster with per-member role controls.
#[component]
async fn team_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Your team")
                card_description("Everyone with access to this workspace.")
            )
            card_content(
                <div class="flex flex-col gap-4">
                    // Only the first member has a portrait; the others fall
                    // back to their initials, which is also what shows while
                    // an image is still loading.
                    for (name, email, role, initials, portrait) in [
                        ("Ada Lovelace", "ada@example.com", "Owner", "AL", true),
                        ("Grace Hopper", "grace@example.com", "Member", "GH", false),
                        ("Alan Turing", "alan@example.com", "Member", "AT", false),
                    ] {
                        <div class="flex items-center justify-between gap-4">
                            <div class="flex min-w-0 items-center gap-3">
                                avatar(
                                    size: AvatarSize::Sm,
                                    if portrait {
                                        avatar_image(attrs: attributes! { src=(PORTRAIT) })
                                    }
                                    avatar_fallback((initials))
                                )
                                <div class="min-w-0">
                                    <p class="truncate text-sm font-medium">(name)</p>
                                    <p class="truncate text-xs text-muted-foreground">
                                        (email)
                                    </p>
                                </div>
                            </div>
                            button(
                                size: ButtonSize::Sm,
                                variant: ButtonVariant::Outline,
                                (role)
                                icon(data: iconify_icon!("feather:chevron-down"))
                            )
                        </div>
                    }
                </div>
            )
            card_footer(
                button(
                    size: ButtonSize::Sm,
                    variant: ButtonVariant::Secondary,
                    icon(data: iconify_icon!("feather:user-plus"))
                    "Invite member"
                )
            )
        )
    }
}

/// The button family: variants, sizes, and the disabled state at a glance.
#[component]
async fn buttons_card() -> Result {
    view! {
        card(
            card_content(
                <div class="flex flex-col gap-3">
                    <div class="flex flex-wrap items-center gap-2">
                        for (variant, name) in [
                            (ButtonVariant::Primary, "Primary"),
                            (ButtonVariant::Secondary, "Secondary"),
                            (ButtonVariant::Outline, "Outline"),
                            (ButtonVariant::Ghost, "Ghost"),
                            (ButtonVariant::Destructive, "Destructive"),
                        ] {
                            button(size: ButtonSize::Sm, variant: variant, (name))
                        }
                    </div>
                    <div class="flex flex-wrap items-center gap-2">
                        button(size: ButtonSize::Sm, "Small")
                        button(size: ButtonSize::Md, "Medium")
                        button(size: ButtonSize::Lg, "Large")
                        button(
                            size: ButtonSize::Icon,
                            variant: ButtonVariant::Outline,
                            icon(data: iconify_icon!("feather:plus"), label: "Add item")
                        )
                        button(
                            attrs: attributes! { disabled="" },
                            spinner()
                            "Saving..."
                        )
                    </div>
                </div>
            )
        )
    }
}

/// A sign-in form pairing labeled inputs with a full-width submit.
#[component]
async fn sign_in_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Sign in")
                card_description("Use your work email to continue.")
            )
            card_content(
                <form class="flex flex-col gap-4">
                    <div class="flex flex-col gap-2">
                        label(attrs: attributes! { for="email" }, "Email")
                        input(
                            attrs: attributes! { id="email" type="email" placeholder="you@example.com" }
                        )
                    </div>
                    <div class="flex flex-col gap-2">
                        label(attrs: attributes! { for="password" }, "Password")
                        input(attrs: attributes! { id="password" type="password" })
                    </div>
                </form>
            )
            card_footer(button(attrs: attributes! { class="w-full" }, "Sign in"))
        )
    }
}

/// A creation form mixing an input, a select, and a confirming footer.
#[component]
async fn project_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Create project")
                card_description("Deploys go to the region you pick here.")
            )
            card_content(
                <form class="flex flex-col gap-4">
                    <div class="flex flex-col gap-2">
                        label(attrs: attributes! { for="project-name" }, "Name")
                        input(
                            attrs: attributes! { id="project-name" placeholder="my-app" }
                        )
                    </div>
                    <div class="flex flex-col gap-2">
                        label(attrs: attributes! { for="region" }, "Region")
                        select(
                            attrs: attributes! { id="region" },
                            <option>"eu-central-1"</option>
                            <option>"us-east-1"</option>
                            <option>"ap-southeast-2"</option>
                        )
                    </div>
                </form>
            )
            card_footer(
                attrs: attributes! { class="justify-end" },
                button(variant: ButtonVariant::Ghost, "Cancel")
                button("Create project")
            )
        )
    }
}

/// A branch switcher, rendered open so the menu shows on the page.
#[component]
async fn branches_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Branches")
                card_description("Switch the branch this preview builds from.")
            )
            card_content(
                dropdown_menu(
                    attrs: attributes! { open="" },
                    // The trigger takes any content; this one borrows the
                    // outline button's looks and adds a flipping chevron.
                    dropdown_menu_trigger(
                        attrs: attributes! {
                            class=(button_variants(
                                ButtonVariant::Outline,
                                ButtonSize::Sm,
                            ))
                        },
                        "feature/showcase"
                        icon(
                            data: iconify_icon!("feather:chevron-down"),
                            attrs: attributes! { class="transition-transform group-open:rotate-180" }
                        )
                    )
                    dropdown_menu_content(
                        dropdown_menu_label("Switch branch")
                        dropdown_menu_item("main")
                        dropdown_menu_item("feature/showcase")
                        dropdown_menu_item("feature/dark-mode")
                        dropdown_menu_separator()
                        // A submenu opens its own panel beside this row; it is
                        // rendered open here so the page shows it.
                        dropdown_menu_sub(
                            attrs: attributes! { open="" },
                            dropdown_menu_sub_trigger("Checkout tag")
                            dropdown_menu_sub_content(
                                dropdown_menu_item("v1.2.0")
                                dropdown_menu_item("v1.1.0")
                                dropdown_menu_item("v1.0.0")
                            )
                        )
                        dropdown_menu_item("Create branch...")
                    )
                )
                // The open menu and submenu float over the flow; reserve
                // their room so they stay within the card.
                <div class="h-64"></div>
            )
        )
    }
}

/// A pricing card with a feature list and an upgrade action.
#[component]
async fn upgrade_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Pro")
                card_description("For teams shipping to production.")
            )
            card_content(
                <p>
                    <span class="text-3xl font-bold">"$24"</span>
                    <span class="text-sm text-muted-foreground">" / month"</span>
                </p>
                <ul class="mt-4 flex flex-col gap-2 text-sm">
                    for feature in [
                        "Unlimited projects",
                        "Preview deployments",
                        "Audit log",
                        "Priority support",
                    ] {
                        <li class="flex items-center gap-2">
                            icon(data: iconify_icon!("feather:check"))
                            (feature)
                        </li>
                    }
                </ul>
            )
            card_footer(button(attrs: attributes! { class="w-full" }, "Upgrade"))
        )
    }
}

/// A dark-scheme demo: the `dark` class on the wrapper restyles everything
/// inside it, because components reference theme tokens instead of raw colors.
#[component]
async fn deploy_card() -> Result {
    view! {
        <div class="dark">
            card(
                card_header(
                    card_title("Deployment ready")
                    card_description(
                        "topcoat-ui@0.4.2 built in 38s and passed all checks."
                    )
                )
                card_footer(
                    button(size: ButtonSize::Sm, "Promote to production")
                    button(
                        size: ButtonSize::Sm,
                        variant: ButtonVariant::Ghost,
                        "View logs"
                    )
                )
            )
        </div>
    }
}

/// A confirmation card pairing a quiet dismiss with a destructive commit.
#[component]
async fn delete_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Delete workspace")
                card_description(
                    "This permanently removes the workspace and all of its data."
                )
            )
            card_footer(
                attrs: attributes! { class="justify-end" },
                button(variant: ButtonVariant::Ghost, "Cancel")
                // The commit goes through an alert dialog, so it takes a
                // deliberate answer rather than one stray click.
                <a
                    href="?dialog=delete"
                    class=(button_variants(
                        ButtonVariant::Destructive,
                        ButtonSize::Md,
                    ))
                >
                    "Delete workspace"
                </a>
            )
        )
    }
}

/// The alert dialog behind the delete action: it asks the question and offers
/// nothing but the two answers to it.
#[component]
async fn delete_dialog(open: bool) -> Result {
    // Bound out here rather than inline: a hyphenated attribute name inside
    // an `attributes!` nested in a `view!` currently trips `topcoat fmt`.
    let labels = attributes! {
        aria-labelledby="delete-title"
        aria-describedby="delete-description"
    };

    view! {
        alert_dialog(
            open: open,
            attrs: labels,
            dialog_content(
                dialog_header(
                    dialog_title(
                        attrs: attributes! { id="delete-title" },
                        "Delete this workspace?"
                    )
                    dialog_description(
                        attrs: attributes! { id="delete-description" },
                        "Its projects, deploys, and audit log go with it. This \
                         cannot be undone."
                    )
                )
                dialog_footer(
                    <a
                        href="/"
                        class=(button_variants(
                            ButtonVariant::Ghost,
                            ButtonSize::Md,
                        ))
                    >
                        "Keep the workspace"
                    </a>
                    button(variant: ButtonVariant::Destructive, "Delete")
                )
            )
        )
    }
}

/// Environment statuses told through the badge variants.
#[component]
async fn status_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Deployment status")
                card_description("Every environment at a glance.")
            )
            card_content(
                <div class="flex flex-col gap-3">
                    for (env, status, variant) in [
                        ("production", "Live", BadgeVariant::Primary),
                        ("staging", "Building", BadgeVariant::Secondary),
                        ("preview/pr-142", "Queued", BadgeVariant::Outline),
                        ("legacy-api", "Failed", BadgeVariant::Destructive),
                    ] {
                        <div class="flex items-center justify-between gap-4">
                            <p class="truncate font-mono text-sm">(env)</p>
                            badge(variant: variant, (status))
                        </div>
                    }
                    <div class="flex items-center justify-between gap-4">
                        <p class="truncate font-mono text-sm">"preview/pr-143"</p>
                        <p
                            class="flex items-center gap-1.5 text-xs text-muted-foreground"
                        >
                            spinner()
                            "Deploying..."
                        </p>
                    </div>
                    <div class="flex flex-col gap-2 border-t border-border pt-4">
                        <div class="flex items-center justify-between gap-4">
                            <p class="text-sm text-muted-foreground">
                                "Rolling out to production"
                            </p>
                            <p class="text-sm font-medium">"62%"</p>
                        </div>
                        progress(value: 62.0)
                    </div>
                </div>
            )
            card_footer(
                <p class="text-sm text-muted-foreground">"Rolled out with"</p>
                // Anything can borrow a badge's looks: `badge_variants`
                // returns the class string for a variant.
                <a href="#changelog" class=(badge_variants(BadgeVariant::Outline))>
                    "v2.0.4"
                </a>
            )
        )
    }
}

/// A share sheet with a copyable link.
#[component]
async fn share_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Share this document")
                card_description("Anyone with the link can view it.")
            )
            card_content(
                <div class="flex items-center gap-2">
                    <p
                        class="min-w-0 flex-1 truncate rounded-lg border border-border \
                            px-3 py-2 text-sm text-muted-foreground"
                    >
                        "https://topcoat.dev/d/quickstart"
                    </p>
                    button(
                        size: ButtonSize::Icon,
                        variant: ButtonVariant::Outline,
                        icon(data: iconify_icon!("feather:copy"), label: "Copy link")
                    )
                </div>
            )
        )
    }
}

/// Notification settings mixing unchecked, checked, and disabled checkboxes.
#[component]
async fn settings_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Notification settings")
                card_description("Pick what lands in your inbox.")
            )
            card_content(
                <div class="flex flex-col gap-3">
                    <div class="flex items-center gap-2">
                        checkbox(attrs: attributes! { id="notify-deploys" checked="" })
                        label(
                            attrs: attributes! { for="notify-deploys" },
                            "Deploy results"
                        )
                    </div>
                    <div class="flex items-center gap-2">
                        checkbox(attrs: attributes! { id="notify-mentions" })
                        label(attrs: attributes! { for="notify-mentions" }, "Mentions")
                    </div>
                    <div class="flex items-center gap-2">
                        checkbox(
                            attrs: attributes! { id="notify-digest" checked="" disabled="" }
                        )
                        label(
                            attrs: attributes! { for="notify-digest" class="opacity-50" },
                            "Weekly digest (managed by your org)"
                        )
                    </div>
                    <div class="flex flex-col gap-3 border-t border-border pt-4">
                        <div class="flex items-center justify-between gap-4">
                            label(
                                attrs: attributes! { for="notify-push" },
                                "Push notifications"
                            )
                            switch(attrs: attributes! { id="notify-push" checked="" })
                        </div>
                        <div class="flex items-center justify-between gap-4">
                            label(
                                attrs: attributes! { for="notify-quiet" },
                                "Quiet hours"
                            )
                            switch(attrs: attributes! { id="notify-quiet" })
                        </div>
                    </div>
                </div>
            )
        )
    }
}

/// A feedback form pairing a labeled textarea with a submit action.
#[component]
async fn feedback_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Send feedback")
                card_description("What should we improve next?")
            )
            card_content(
                <form class="flex flex-col gap-2">
                    label(attrs: attributes! { for="feedback" }, "Your feedback")
                    textarea(
                        attrs: attributes! {
                            id="feedback"
                            name="feedback"
                            placeholder="The dropdown menu could..."
                        }
                    )
                </form>
            )
            card_footer(
                attrs: attributes! { class="justify-end" },
                button("Send feedback")
            )
        )
    }
}

/// A settings card whose action opens [`rename_dialog`]. Nothing about the
/// trigger is special: opening the dialog is navigating to the URL the page
/// renders it open for.
#[component]
async fn rename_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Project settings")
                card_description("Rename the project without redeploying it.")
            )
            card_content(
                <div class="flex items-center justify-between gap-4">
                    <p class="truncate font-mono text-sm">"topcoat-ui"</p>
                    <a
                        href="?dialog=rename"
                        class=(button_variants(
                            ButtonVariant::Outline,
                            ButtonSize::Sm,
                        ))
                    >
                        "Rename"
                    </a>
                </div>
            )
        )
    }
}

/// The dialog over the page, shown while `open` is true.
///
/// Closing it is navigating back to the page that renders it closed, which
/// both the corner button and "Cancel" do.
#[component]
async fn rename_dialog(open: bool) -> Result {
    view! {
        dialog(
            open: open,
            dialog_content(
                // The panel is positioned, so a close control can sit in one
                // of its corners.
                <a
                    href="/"
                    class=(class!(
                        button_variants(
                            ButtonVariant::Ghost,
                            ButtonSize::Icon,
                        ),
                        "absolute top-3 right-3",
                    ))
                >
                    icon(data: iconify_icon!("feather:x"), label: "Close")
                </a>
                dialog_header(
                    dialog_title("Rename project")
                    dialog_description(
                        "The project keeps its deploy URLs; only the name \
                         shown to your team changes."
                    )
                )
                <form class="flex flex-col gap-2">
                    label(attrs: attributes! { for="rename" }, "Name")
                    input(attrs: attributes! { id="rename" value="topcoat-ui" })
                </form>
                dialog_footer(
                    <a
                        href="/"
                        class=(button_variants(
                            ButtonVariant::Ghost,
                            ButtonSize::Md,
                        ))
                    >
                        "Cancel"
                    </a>
                    button("Save")
                )
            )
        )
    }
}

/// The panels the overview card tabs between, and the labels of their
/// triggers.
const TABS: [(&str, &str); 3] = [
    ("overview", "Overview"),
    ("activity", "Activity"),
    ("settings", "Settings"),
];

/// A card that tabs between panels.
///
/// Which panel shows is in the URL, so each trigger is a link and only the
/// panel being read is rendered.
#[component]
async fn overview_card(tab: &str) -> Result {
    view! {
        card(
            card_header(
                card_title("Project")
                card_description("Everything about topcoat-ui in one place.")
            )
            card_content(
                tabs(
                    tabs_list(
                        for (value, text) in TABS {
                            tabs_trigger(
                                active: value == tab,
                                attrs: attributes! { href=(format!("?tab={value}")) },
                                (text)
                            )
                        }
                    )
                    tabs_content(
                        <p class="text-sm text-muted-foreground">
                            (match tab {
                                "activity" => "Grace deployed to production 2 hours ago.",
                                "settings" => {
                                    "The project is on the Pro plan, in eu-central-1."
                                }
                                _ => "Eight deploys this week, all of them green.",
                            })
                        </p>
                    )
                )
            )
        )
    }
}

/// Two things that show on hover: a tooltip carrying a few words, and a hover
/// card carrying a view.
#[component]
async fn hints_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Hover to reveal")
                card_description("A hint for the button, a profile for the mention.")
            )
            card_content(
                <div class="flex items-center gap-4">
                    tooltip(
                        button(
                            size: ButtonSize::Icon,
                            variant: ButtonVariant::Outline,
                            icon(
                                data: iconify_icon!("feather:copy"),
                                label: "Copy the deploy URL"
                            )
                        )
                        tooltip_content("Copy the deploy URL")
                    )
                    hover_card(
                        <a href="#ada" class="text-sm font-medium underline">"@ada"</a>
                        hover_card_content(
                            <div class="flex items-center gap-3">
                                avatar(
                                    size: AvatarSize::Sm,
                                    avatar_image(attrs: attributes! { src=(PORTRAIT) })
                                    avatar_fallback("AL")
                                )
                                <div class="min-w-0">
                                    <p class="truncate text-sm font-medium">"Ada Lovelace"</p>
                                    <p class="truncate text-xs text-muted-foreground">
                                        "Owner"
                                    </p>
                                </div>
                            </div>
                            <p class="text-xs text-muted-foreground">
                                "Joined in 2024. Deploys on Fridays anyway."
                            </p>
                        )
                    )
                </div>
            )
        )
    }
}

/// The sheet behind the deployments table's "Filters" link: a panel along the
/// right edge, holding what a dialog would be too small for.
#[component]
async fn filters_sheet(open: bool) -> Result {
    view! {
        sheet(
            open: open,
            sheet_content(
                side: SheetSide::Right,
                dialog_header(
                    dialog_title("Filters")
                    dialog_description("Narrow the deployments in the table.")
                )
                <div class="flex flex-col gap-4">
                    <div class="flex flex-col gap-2">
                        label(attrs: attributes! { for="filter-env" }, "Environment")
                        select(
                            attrs: attributes! { id="filter-env" },
                            <option>"All environments"</option>
                            <option>"production"</option>
                            <option>"staging"</option>
                        )
                    </div>
                    radio_group(
                        for (value, text) in [
                            ("any", "Any status"),
                            ("live", "Live"),
                            ("failed", "Failed"),
                        ] {
                            <div class="flex items-center gap-2">
                                radio_group_item(
                                    attrs: attributes! {
                                        id=(format!("filter-{value}"))
                                        name="status"
                                        value=(value)
                                        checked=(value == "any")
                                    }
                                )
                                label(
                                    attrs: attributes! { for=(format!("filter-{value}")) },
                                    (text)
                                )
                            </div>
                        }
                    )
                </div>
                dialog_footer(
                    attrs: attributes! { class="mt-auto" },
                    <a
                        href="/"
                        class=(button_variants(
                            ButtonVariant::Ghost,
                            ButtonSize::Md,
                        ))
                    >
                        "Cancel"
                    </a>
                    button("Apply filters")
                )
            )
        )
    }
}

/// A plan picker, built on a radio group.
#[component]
async fn plan_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Billing plan")
                card_description("Pick the plan this workspace runs on.")
            )
            card_content(
                radio_group(
                    // The name the options share is what has the browser
                    // let go of one when another is picked.
                    for (value, name, price, picked) in [
                        ("starter", "Starter", "Free", false),
                        ("pro", "Pro", "$24 / month", true),
                        ("scale", "Scale", "$96 / month", false),
                    ] {
                        <div class="flex items-center gap-2">
                            radio_group_item(
                                attrs: attributes! { id=(value) name="plan" value=(value) checked=(picked) }
                            )
                            label(
                                attrs: attributes! { for=(value) class="flex-1" },
                                (name)
                            )
                            <p class="text-sm text-muted-foreground">(price)</p>
                        </div>
                    }
                )
            )
        )
    }
}

/// A toolbar of toggles: a segmented control where picking one lets go of the
/// rest, and formatting toggles that press on their own.
#[component]
async fn toolbar_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Report")
                card_description("The range it covers, and how it reads.")
            )
            card_content(
                <div class="flex flex-col gap-4">
                    toggle_group(
                        for (value, text, picked) in [
                            ("day", "Day", false),
                            ("week", "Week", true),
                            ("month", "Month", false),
                        ] {
                            toggle(
                                kind: ToggleKind::Exclusive,
                                size: ToggleSize::Sm,
                                attrs: attributes! { name="range" value=(value) checked=(picked) },
                                (text)
                            )
                        }
                    )
                    <div class="flex items-center gap-1">
                        for (name, data, text, pressed) in [
                            ("bold", iconify_icon!("feather:bold"), "Bold", true),
                            ("italic", iconify_icon!("feather:italic"), "Italic", false),
                            (
                                "underline",
                                iconify_icon!("feather:underline"),
                                "Underline",
                                false,
                            ),
                        ] {
                            toggle(
                                size: ToggleSize::Sm,
                                attrs: attributes! { name=(name) checked=(pressed) },
                                icon(data: data, label: text)
                            )
                        }
                    </div>
                </div>
            )
        )
    }
}

/// A FAQ whose answers fold away, one open at a time.
#[component]
async fn faq_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Questions")
                card_description("The rest is in the docs.")
            )
            card_content(
                accordion(
                    // The name the sections share is what closes the open one
                    // when another is opened.
                    for (question, answer, open) in [
                        (
                            "Where do the components live?",
                            "In your own source tree, under the components \
                             directory you picked.",
                            true,
                        ),
                        (
                            "Can I edit them?",
                            "They are yours: restyle, rewrite, and extend them \
                             like any other file.",
                            false,
                        ),
                        (
                            "How do updates work?",
                            "`topcoat ui list` marks what the registry has \
                             changed since you added it.",
                            false,
                        ),
                    ] {
                        accordion_item(
                            attrs: attributes! { name="faq" open=(open) },
                            accordion_trigger((question))
                            accordion_content((answer))
                        )
                    }
                )
            )
        )
    }
}

/// Two notices: the neutral variant and the destructive one.
#[component]
async fn alerts_card() -> Result {
    view! {
        card(
            card_content(
                <div class="flex flex-col gap-3">
                    // The leading icon is an ordinary child: the alert lays
                    // out a column for it only when one is there.
                    alert(
                        icon(data: iconify_icon!("feather:info"))
                        alert_title("Scheduled maintenance")
                        alert_description(
                            "The dashboard is read-only on Sunday, 02:00 to \
                             04:00 UTC."
                        )
                    )
                    alert(
                        variant: AlertVariant::Destructive,
                        icon(data: iconify_icon!("feather:alert-triangle"))
                        alert_title("Build failed")
                        alert_description(
                            "legacy-api could not resolve its dependencies."
                        )
                    )
                </div>
            )
        )
    }
}

/// The deployments the table pages through, two to a page.
const DEPLOYMENTS: [(&str, &str, &str, BadgeVariant); 16] = [
    ("a1b2c3d", "production", "Live", BadgeVariant::Primary),
    ("9f8e7d6", "staging", "Building", BadgeVariant::Secondary),
    ("4c5b6a7", "preview/pr-142", "Queued", BadgeVariant::Outline),
    ("2e1d0c9", "legacy-api", "Failed", BadgeVariant::Destructive),
    ("7b6a5f4", "production", "Live", BadgeVariant::Primary),
    ("3d2c1b0", "preview/pr-139", "Queued", BadgeVariant::Outline),
    ("8e7d6c5", "staging", "Live", BadgeVariant::Primary),
    (
        "1a0b9c8",
        "preview/pr-137",
        "Failed",
        BadgeVariant::Destructive,
    ),
    ("5c4b3a2", "production", "Live", BadgeVariant::Primary),
    ("0f9e8d7", "staging", "Building", BadgeVariant::Secondary),
    ("6a5b4c3", "preview/pr-131", "Queued", BadgeVariant::Outline),
    ("d4c3b2a", "legacy-api", "Failed", BadgeVariant::Destructive),
    ("e5f6a7b", "production", "Live", BadgeVariant::Primary),
    ("c3b2a1f", "preview/pr-128", "Queued", BadgeVariant::Outline),
    ("b7a6d5e", "staging", "Live", BadgeVariant::Primary),
    (
        "f2e1d0c",
        "preview/pr-124",
        "Failed",
        BadgeVariant::Destructive,
    ),
];

/// How many deployments one page of the table holds.
const PER_PAGE: usize = 2;

/// How many pages the deployments fill.
const PAGES: usize = DEPLOYMENTS.len() / PER_PAGE;

/// A table of deployments, paginated underneath.
///
/// The `page` comes from the URL, so the links below the table are what
/// change both the rows and which page reads as the current one.
#[component]
async fn deployments_card(page: usize) -> Result {
    let rows = &DEPLOYMENTS[(page - 1) * PER_PAGE..page * PER_PAGE];

    view! {
        card(
            card_header(
                card_title("Deployments")
                card_description("The last builds of this project.")
            )
            card_content(
                <div class="flex items-center justify-between gap-4">
                    <p class="text-sm text-muted-foreground">"All environments"</p>
                    // What the sheet holds would not fit a dialog, so it
                    // comes in from the edge instead.
                    <a
                        href="?dialog=filters"
                        class=(button_variants(
                            ButtonVariant::Outline,
                            ButtonSize::Sm,
                        ))
                    >
                        icon(data: iconify_icon!("feather:filter"))
                        "Filters"
                    </a>
                </div>
            )
            // The card pads its sections rather than itself, so the table can
            // span its full width; the table's own padding lines the cells up
            // with the sections above and below it.
            table(
                attrs: attributes! { class="px-3" },
                table_caption("Deployments of the last 24 hours.")
                table_header(
                    table_row(
                        table_head("Commit")
                        table_head("Environment")
                        table_head("Status")
                    )
                )
                table_body(
                    for (commit, env, status, variant) in rows {
                        table_row(
                            table_cell(
                                attrs: attributes! { class="font-mono" },
                                (commit)
                            )
                            table_cell((env))
                            table_cell(badge(variant: *variant, (status)))
                        )
                    }
                )
                table_footer(
                    table_row(
                        table_cell(attrs: attributes! { colspan="2" }, "Total")
                        table_cell((format!("{} deployments", DEPLOYMENTS.len())))
                    )
                )
            )
            card_footer(
                attrs: attributes! { class="justify-center" },
                pagination(
                    pagination_content(
                        pagination_item(
                            pagination_previous(
                                attrs: attributes! { href=(page_href(page.saturating_sub(1).max(1))) }
                            )
                        )
                        for number in 1..=PAGES {
                            if listed(number, page) {
                                pagination_item(
                                    pagination_link(
                                        active: number == page,
                                        attrs: attributes! { href=(page_href(number)) },
                                        (number)
                                    )
                                )
                            } else if listed(number - 1, page) {
                                // The first page left out of a run stands for
                                // the whole run.
                                pagination_item(pagination_ellipsis())
                            }
                        }
                        pagination_item(
                            pagination_next(
                                attrs: attributes! { href=(page_href((page + 1).min(PAGES))) }
                            )
                        )
                    )
                )
            )
        )
    }
}

/// The URL of one page of the deployments table.
fn page_href(page: usize) -> String {
    format!("?page={page}")
}

/// Whether `number` gets a link of its own while `page` is the one being
/// read: the first page, the last one, and the current one do, and the runs
/// left between them collapse into an ellipsis.
///
/// Listing the current page's neighbours too, as a roomier pagination would,
/// grows the row past the width of a card in this masonry. The pagination
/// wraps rather than overflowing when that happens, but stepping one page at
/// a time is what "Previous" and "Next" are already for.
fn listed(number: usize, page: usize) -> bool {
    number == 1 || number == PAGES || number == page
}

/// A documentation page header: the trail to it, and the shortcuts it lists.
#[component]
async fn docs_card() -> Result {
    view! {
        card(
            card_header(
                breadcrumb(
                    breadcrumb_list(
                        breadcrumb_item(
                            breadcrumb_link(attrs: attributes! { href="#docs" }, "Docs")
                        )
                        breadcrumb_separator()
                        // The steps between are collapsed into an ellipsis.
                        breadcrumb_item(breadcrumb_ellipsis())
                        breadcrumb_separator()
                        breadcrumb_item(
                            breadcrumb_link(
                                attrs: attributes! { href="#components" },
                                "Components"
                            )
                        )
                        breadcrumb_separator()
                        breadcrumb_item(breadcrumb_page("Dialog"))
                    )
                )
                card_title("Dialog")
                card_description("A panel over the page for a single task.")
            )
            card_content(
                <div class="flex flex-col gap-3">
                    for (action, keys) in [
                        ("Search the docs", ["Ctrl", "K"]),
                        ("Copy the snippet", ["Ctrl", "C"]),
                    ] {
                        <div class="flex items-center justify-between gap-4">
                            <p class="text-sm text-muted-foreground">(action)</p>
                            kbd_group(
                                for key in keys {
                                    kbd((key))
                                }
                            )
                        </div>
                    }
                </div>
            )
        )
    }
}

/// The team roster again, in the shape it has before the data arrives.
#[component]
async fn loading_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Your team")
                card_description("Everyone with access to this workspace.")
            )
            card_content(
                // The skeletons take the size of what they stand in for, so
                // the card keeps its height once the roster lands.
                <div class="flex flex-col gap-4">
                    for _ in 0..3 {
                        <div class="flex items-center gap-3">
                            skeleton(attrs: attributes! { class="size-8 rounded-full" })
                            <div class="flex flex-1 flex-col gap-1.5">
                                skeleton(attrs: attributes! { class="h-3.5 w-28" })
                                skeleton(attrs: attributes! { class="h-3 w-40" })
                            </div>
                            skeleton(attrs: attributes! { class="h-8 w-20 rounded-md" })
                        </div>
                    }
                </div>
            )
        )
    }
}

/// A footer strip showing both orientations of the separator: a rule under
/// the blurb, and rules between the links.
#[component]
async fn links_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Topcoat UI")
                card_description("An open-source component library.")
            )
            card_content(
                separator()
                // The row's height is what gives the vertical rules theirs.
                <div class="mt-4 flex h-5 items-center gap-3 text-sm">
                    <a href="#docs" class="text-muted-foreground hover:text-foreground">
                        "Docs"
                    </a>
                    separator(orientation: SeparatorOrientation::Vertical)
                    <a
                        href="#registry"
                        class="text-muted-foreground hover:text-foreground"
                    >
                        "Registry"
                    </a>
                    separator(orientation: SeparatorOrientation::Vertical)
                    <a
                        href="#source"
                        class="text-muted-foreground hover:text-foreground"
                    >
                        "Source"
                    </a>
                </div>
            )
        )
    }
}

/// An inbox digest with a bulk action in the footer.
#[component]
async fn notifications_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Notifications")
                card_description("You have 3 unread messages.")
            )
            card_content(
                <div class="flex flex-col gap-4">
                    for (title, time) in [
                        ("Your invoice for June is ready.", "2h ago"),
                        ("grace@example.com joined your team.", "5h ago"),
                        ("Deployment to production succeeded.", "1d ago"),
                    ] {
                        <div class="flex items-start gap-3">
                            <span
                                class="mt-1.5 size-2 shrink-0 rounded-full bg-primary"
                            ></span>

                            <div class="min-w-0">
                                <p class="text-sm">(title)</p>
                                <p class="text-xs text-muted-foreground">(time)</p>
                            </div>
                        </div>
                    }
                </div>
            )
            card_footer(
                button(
                    size: ButtonSize::Sm,
                    variant: ButtonVariant::Outline,
                    icon(data: iconify_icon!("feather:check"))
                    "Mark all as read"
                )
            )
        )
    }
}
