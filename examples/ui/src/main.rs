mod components;

use components::badge::{BadgeVariant, badge, badge_variants};
use components::button::{ButtonSize, ButtonVariant, button, button_variants};
use components::card::{
    card, card_content, card_description, card_footer, card_header, card_title,
};
use components::checkbox::checkbox;
use components::dropdown_menu::{
    dropdown_menu, dropdown_menu_content, dropdown_menu_item, dropdown_menu_label,
    dropdown_menu_separator, dropdown_menu_sub, dropdown_menu_sub_content,
    dropdown_menu_sub_trigger, dropdown_menu_trigger,
};
use components::input::input;
use components::label::label;
use components::progress::progress;
use components::select::select;
use components::spinner::spinner;
use components::switch::switch;
use components::textarea::textarea;
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    font::fontsource::fontsource_font,
    icon::{icon, iconify::iconify_icon},
    router::{Router, RouterBuilderDiscoverExt, page},
    tailwind,
    view::{View, attributes, component, view},
};

#[tokio::main]
async fn main() {
    let router = Router::builder()
        .assets(AssetBundle::load().unwrap())
        .discover()
        .build();

    topcoat::start(router).await.unwrap();
}

#[page("/")]
async fn home() -> Result {
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

                    // A masonry of small, self-contained demos. Each cell is
                    // a `demo` (built from installed components) or a
                    // `coming_soon` placeholder for one not yet in the
                    // registry.
                    <div class="mt-14 columns-1 gap-4 sm:columns-2 xl:columns-3">
                        demo(team_card())
                        demo(buttons_card())
                        demo(sign_in_card())
                        demo(upgrade_card())
                        demo(deploy_card())
                        coming_soon(name: "Tabs")
                        demo(delete_card())
                        demo(status_card())
                        demo(branches_card())
                        demo(share_card())
                        demo(settings_card())
                        demo(project_card())
                        demo(feedback_card())
                        demo(notifications_card())
                        coming_soon(name: "Dialog")
                        coming_soon(name: "Avatar")
                    </div>
                </main>
            </body>
        </html>
    }
}

/// A masonry cell: keeps a demo from splitting across columns.
#[component]
async fn demo(child: View) -> Result {
    view! { <div class="mb-4 break-inside-avoid">(child)</div> }
}

/// A placeholder cell for a component that is not in the registry yet.
#[component]
async fn coming_soon(name: &'static str) -> Result {
    view! {
        <div
            class="mb-4 flex break-inside-avoid flex-col items-center justify-center \
                gap-1 rounded-xl border border-dashed border-border px-6 py-10"
        >
            <p class="text-sm font-medium">(name)</p>
            <p class="text-xs text-muted-foreground">"Coming soon"</p>
        </div>
    }
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
                    for (name, email, role) in [
                        ("Ada Lovelace", "ada@example.com", "Owner"),
                        ("Grace Hopper", "grace@example.com", "Member"),
                        ("Alan Turing", "alan@example.com", "Member"),
                    ] {
                        <div class="flex items-center justify-between gap-4">
                            <div class="min-w-0">
                                <p class="truncate text-sm font-medium">(name)</p>
                                <p class="truncate text-xs text-muted-foreground">
                                    (email)
                                </p>
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
                            attrs: attributes! { disabled=(true) },
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
                    attrs: attributes! { open=(true) },
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
                            attrs: attributes! { open=(true) },
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
                button(variant: ButtonVariant::Destructive, "Delete workspace")
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
                        checkbox(
                            attrs: attributes! { id="notify-deploys" checked=(true) }
                        )
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
                            attrs: attributes! { id="notify-digest" checked=(true) disabled=(true) }
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
                            switch(
                                attrs: attributes! { id="notify-push" checked=(true) }
                            )
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
                            >

                            </span>
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
