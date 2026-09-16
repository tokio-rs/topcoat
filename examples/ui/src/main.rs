mod components;

use components::{
    accordion::{accordion, accordion_content, accordion_item, accordion_trigger},
    alert::{AlertVariant, alert, alert_description, alert_title},
    alert_dialog::alert_dialog,
    avatar::{AvatarSize, avatar, avatar_fallback, avatar_image},
    badge::{BadgeVariant, badge, badge_variants},
    breadcrumb::{
        breadcrumb, breadcrumb_ellipsis, breadcrumb_item, breadcrumb_link, breadcrumb_list,
        breadcrumb_page, breadcrumb_separator,
    },
    button::{ButtonSize, ButtonVariant, button, button_variants},
    card::{card, card_content, card_description, card_footer, card_header, card_title},
    checkbox::checkbox,
    dialog::{
        dialog, dialog_content, dialog_description, dialog_footer, dialog_header, dialog_title,
    },
    dropdown_menu::{
        dropdown_menu, dropdown_menu_content, dropdown_menu_item, dropdown_menu_label,
        dropdown_menu_separator, dropdown_menu_sub, dropdown_menu_sub_content,
        dropdown_menu_sub_trigger, dropdown_menu_trigger,
    },
    hover_card::{hover_card, hover_card_content},
    input::input,
    kbd::{kbd, kbd_group},
    label::label,
    pagination::{
        pagination, pagination_content, pagination_ellipsis, pagination_item, pagination_link,
        pagination_next, pagination_previous,
    },
    progress::progress,
    radio_group::{radio_group, radio_group_item},
    select::select,
    separator::{SeparatorOrientation, separator},
    sheet::{sheet, sheet_content},
    sidebar::{
        SidebarCollapsible, SidebarMenuButtonSize, sidebar, sidebar_content, sidebar_footer,
        sidebar_group, sidebar_group_action, sidebar_group_content, sidebar_group_label,
        sidebar_header, sidebar_inset, sidebar_menu, sidebar_menu_badge, sidebar_menu_button,
        sidebar_menu_item, sidebar_menu_sub, sidebar_menu_sub_button, sidebar_menu_sub_item,
        sidebar_provider, sidebar_separator, sidebar_trigger,
    },
    skeleton::skeleton,
    spinner::spinner,
    switch::switch,
    table::{
        table, table_body, table_caption, table_cell, table_footer, table_head, table_header,
        table_row,
    },
    tabs::{tabs, tabs_content, tabs_list, tabs_trigger},
    textarea::textarea,
    toggle::{ToggleKind, ToggleSize, toggle, toggle_group},
    tooltip::{tooltip, tooltip_content},
};
use topcoat::{
    Result,
    asset::{Asset, AssetBundle, RouterBuilderAssetExt, asset},
    context::Cx,
    font::fontsource::fontsource_font,
    icon::{icon, iconify::iconify_icon},
    router::{Router, RouterBuilderDiscoverExt, page},
    runtime::{Event, RouterBuilderRuntimeExt, Signal, expr, shard, signal},
    tailwind,
    view::{Child, View, attributes, class, component, view},
};

/// A stand-in portrait for the workspace's owner, served from the example's
/// own asset bundle.
const PORTRAIT: Asset = asset!("./portrait.svg");

/// The pages this one links out to: the framework's documentation, its
/// source, and the registry the components here were added from.
const CRATE: &str = "https://crates.io/crates/topcoat";
const DOCS: &str = "https://docs.rs/topcoat";
const REPOSITORY: &str = "https://github.com/tokio-rs/topcoat";
const REGISTRY: &str = "https://github.com/tokio-rs/topcoat/tree/main/crates/topcoat-ui/registry";

#[tokio::main]
async fn main() {
    let router = Router::builder()
        .runtime()
        .assets(AssetBundle::load().unwrap())
        .discover()
        .build();

    topcoat::start(router).await.unwrap();
}

/// The tab values and labels. The first is selected when the page opens.
const TABS: [(&str, &str); 3] = [
    ("overview", "Overview"),
    ("activity", "Activity"),
    ("settings", "Settings"),
];

/// How many rows one page of the deployments table holds.
const PER_PAGE: usize = 3;

/// The statuses a deployment can be in, and the badge variant each shows in.
const STATUSES: [(&str, BadgeVariant); 4] = [
    ("Live", BadgeVariant::Primary),
    ("Building", BadgeVariant::Secondary),
    ("Queued", BadgeVariant::Outline),
    ("Failed", BadgeVariant::Destructive),
];

/// The branches a preview can build from. The first is the one it builds from
/// until another is picked.
const BRANCHES: [&str; 3] = ["main", "feature/showcase", "feature/dark-mode"];

/// The tags a preview can build from instead of a branch.
const TAGS: [&str; 3] = ["v1.2.0", "v1.1.0", "v1.0.0"];

/// The badge variant the deployment status `status` shows in.
fn status_variant(status: &str) -> BadgeVariant {
    STATUSES
        .iter()
        .find(|(known, _)| *known == status)
        .map_or(BadgeVariant::default(), |(_, variant)| *variant)
}

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    let dark = signal(cx, || false);
    let sidebar_open = signal(cx, || true);
    let mobile_open = signal(cx, || false);

    Ok(view! {
        <!DOCTYPE html>
        <html
            :class=$(if dark.get() { "dark" } else { "" })
            :style=$(if dark.get() {
                "color-scheme: dark"
            } else {
                "color-scheme: light"
            })
        >
            <head>
                <title>"Topcoat UI"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1">
                topcoat::dev::script()
                topcoat::runtime::script()
                topcoat::font::link(font: fontsource_font!(GEIST, host: Asset))
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
            </head>
            // The body's background, text color, and font come from the
            // theme's base layer in styles.css; nothing to set up here.
            <body
                :class=$(if mobile_open.get() { "max-md:overflow-hidden" } else { "" })
            >
                sidebar_provider(
                    app_sidebar(open: &sidebar_open, mobile_open: &mobile_open)
                    sidebar_inset(
                        sidebar_header(
                            sidebar_trigger(
                                open: $(sidebar_open.get()),
                                attrs: attributes! {
                                    class="max-md:hidden"
                                    aria-controls="showcase-sidebar"
                                    @click=$(|_e: Event| sidebar_open.toggle())
                                }
                            )
                            sidebar_trigger(
                                open: $(mobile_open.get()),
                                attrs: attributes! {
                                    class="md:hidden"
                                    aria-controls="showcase-sidebar"
                                    @click=$(|_e: Event| mobile_open.toggle())
                                }
                            )
                            separator(orientation: SeparatorOrientation::Vertical)
                            <span class="text-sm font-medium">"Component library"</span>

                            let theme_label = expr!(
                                if dark.get() {
                                    "Switch to light theme"
                                } else {
                                    "Switch to dark theme"
                                },
                            );

                            button(
                                variant: ButtonVariant::Primary,
                                size: ButtonSize::Md,
                                attrs: attributes! {
                                    type="button"
                                    class="ml-auto"
                                    @click=$(|_e: Event| dark.toggle())
                                    :aria-label=(theme_label.clone())
                                    :title=(theme_label)
                                },
                                <span class="contents" :hidden=$(!dark.get())>
                                    "Light theme"
                                    icon(data: iconify_icon!("lucide:sun"))
                                </span>
                                <span class="contents" :hidden=$(dark.get())>
                                    "Dark theme"
                                    icon(data: iconify_icon!("lucide:moon"))
                                </span>
                            )
                        )
                        <div
                            id="overview"
                            class="mx-auto w-full max-w-6xl scroll-mt-20 px-6 py-12"
                        >
                            <div
                                class="flex flex-col-reverse items-start justify-between gap-6 sm:flex-row"
                            >
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
                                    // Anything can borrow a button's looks:
                                    // `button_variants` returns the class string for a
                                    // variant and size.
                                    <div class="mt-6 flex flex-wrap items-center gap-3">
                                        <a
                                            href=(DOCS)
                                            class=(button_variants(
                                                ButtonVariant::Primary,
                                                ButtonSize::Lg,
                                            ))
                                        >
                                            "Read the docs"
                                            icon(data: iconify_icon!("lucide:arrow-right"))
                                        </a>
                                        <a
                                            href=(REPOSITORY)
                                            class=(button_variants(
                                                ButtonVariant::Outline,
                                                ButtonSize::Lg,
                                            ))
                                        >
                                            "View on GitHub"
                                        </a>
                                    </div>
                                </header>
                            </div>

                            // A masonry of small, self-contained demos, each built
                            // from the installed components. They run from the plainest
                            // components to the ones assembled out of them.
                            <div
                                id="components"
                                class="mt-14 scroll-mt-20 columns-1 gap-4 lg:columns-2 2xl:columns-3"
                            >
                                demo(buttons_card())
                                demo(notices())
                                demo(team_card())
                                demo(status_card())
                                demo(progress_card())
                                demo(id: Some("forms"), form_card())
                                demo(checks_card())
                                demo(switches_card())
                                demo(radios_card())
                                demo(overview_card())
                                demo(faq_card())
                                demo(branches_card())
                                demo(toolbar_card())
                                demo(tooltip_card())
                                demo(hover_card_demo())
                                demo(id: Some("overlays"), dialogs_card())
                                demo(sheet_card())
                                demo(deployments_card())
                                demo(breadcrumbs_card())
                                demo(keyboard_card())
                                demo(skeletons_card())
                                demo(spinner_card())
                            </div>
                        </div>
                    )
                )
            </body>
        </html>
    })
}

/// A masonry cell that keeps a demo from splitting across columns.
#[component]
async fn demo(#[default] id: Option<&str>, child: Child<'_>) -> Result<impl View> {
    Ok(view! { <div id=(id) class="mb-4 scroll-mt-20 break-inside-avoid">(child)</div> })
}

/// The button family: variants, sizes, and states at a glance.
#[component]
async fn buttons_card() -> Result<impl View> {
    Ok(view! {
        card(
            card_header(
                card_title("Buttons")
                card_description("Every variant, size, and state.")
            )
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
                            icon(data: iconify_icon!("lucide:plus"), label: "Add item")
                        )
                    </div>
                    <div class="flex flex-wrap items-center gap-2">
                        button(
                            variant: ButtonVariant::Outline,
                            attrs: attributes! { disabled="" },
                            "Disabled"
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
    })
}

/// Notices standing on their own: an alert is a surface already, so it needs
/// no card under it.
#[component]
async fn notices() -> Result<impl View> {
    Ok(view! {
        <div class="flex flex-col gap-3">
            // The leading icon is an ordinary child: the alert lays out a
            // column for it only when one is there.
            alert(
                icon(data: iconify_icon!("lucide:info"))
                alert_title("Every control is a link or a form")
                alert_description(
                    "Nothing on this page needs scripting; the URL holds what \
                     each one changes."
                )
            )
            alert(
                variant: AlertVariant::Destructive,
                icon(data: iconify_icon!("lucide:triangle-alert"))
                alert_title("The destructive variant")
                alert_description(
                    "For what went wrong, and for what cannot be taken back."
                )
            )
            alert(
                alert_title("Without an icon")
                alert_description(
                    "The title and the text fill the width the icon would \
                     have left them."
                )
            )
        </div>
    })
}

/// The people standing in for a roster: the initials their avatar falls back
/// to, and the role they hold.
const MEMBERS: [(&str, &str, &str, &str); 3] = [
    ("Grace Hopper", "grace@example.com", "GH", "Member"),
    ("Alan Turing", "alan@example.com", "AT", "Member"),
    ("Katherine Johnson", "katherine@example.com", "KJ", "Viewer"),
];

/// A roster: the owner in full, then everyone else with the role they hold.
#[component]
async fn team_card() -> Result<impl View> {
    Ok(view! {
        card(
            card_header(
                card_title("Avatars")
                card_description(
                    "A portrait, initials where there is none, and the role \
                     each one reads in a badge."
                )
            )
            card_content(
                // The owner is the only one with a portrait; the others fall
                // back to their initials, which is also what shows while an
                // image is still loading.
                <div class="flex items-center gap-3">
                    avatar(
                        size: AvatarSize::Lg,
                        avatar_image(attrs: attributes! { src=(PORTRAIT) })
                        avatar_fallback("AL")
                    )
                    <div class="min-w-0 flex-1">
                        <p class="truncate text-sm font-medium">"Ada Lovelace"</p>
                        <p class="truncate text-xs text-muted-foreground">
                            "ada@example.com"
                        </p>
                    </div>
                    badge(variant: BadgeVariant::Secondary, "Owner")
                </div>
                separator(attrs: attributes! { class="my-5" })
                <div class="flex flex-col gap-3">
                    for (name, email, initials, role) in MEMBERS {
                        <div class="flex items-center justify-between gap-3">
                            <div class="flex min-w-0 items-center gap-3">
                                avatar(size: AvatarSize::Sm, avatar_fallback((initials)))
                                <div class="min-w-0">
                                    <p class="truncate text-sm font-medium">(name)</p>
                                    <p class="truncate text-xs text-muted-foreground">
                                        (email)
                                    </p>
                                </div>
                            </div>
                            badge(variant: BadgeVariant::Outline, (role))
                        </div>
                    }
                </div>
            )
        )
    })
}

/// The badge variants with example deployment counts.
#[component]
async fn status_card() -> Result<impl View> {
    Ok(view! {
        card(
            card_header(
                card_title("Badges")
                card_description("Status badges in every variant.")
            )
            card_content(
                <div class="flex flex-col gap-3">
                    for (status, variant) in STATUSES {
                        let count = DEPLOYMENTS
                            .iter()
                            .filter(|(_, _, value)| *value == status)
                            .count();

                        <div class="flex items-center justify-between gap-4">
                            badge(variant: variant, (status))
                            <p class="text-sm text-muted-foreground">
                                (format!("{count} deployments"))
                            </p>
                        </div>
                    }
                </div>
            )
            card_footer(
                <p class="text-sm text-muted-foreground">"Built with Topcoat"</p>
                // Anything can borrow a badge's looks: `badge_variants`
                // returns the class string for a variant.
                <a href=(CRATE) class=(badge_variants(BadgeVariant::Outline))>
                    (format!("v{}", env!("CARGO_PKG_VERSION")))
                </a>
            )
        )
    })
}

/// Determinate and indeterminate progress bars.
#[component]
async fn progress_card(cx: &Cx) -> Result<impl View> {
    let completed = signal(cx, || 62usize);

    Ok(view! {
        card(
            card_header(
                card_title("Progress")
                card_description(
                    "With a known value or an indeterminate amount of work."
                )
            )
            card_content(
                <div class="flex flex-col gap-2">
                    <div class="flex items-center justify-between gap-4">
                        <p class="text-sm text-muted-foreground">
                            "A bar with a value"
                        </p>
                        <p class="text-sm font-medium">
                            $(completed.get())
                            "%"
                        </p>
                    </div>
                    progress(
                        attrs: attributes! { aria-label="Rollout progress" :value=$(completed.get()) }
                    )
                    <div class="flex justify-end gap-2">
                        button(
                            variant: ButtonVariant::Outline,
                            size: ButtonSize::Sm,
                            attrs: attributes! { type="button" @click=$(|_e: Event| completed.set(0)) },
                            "Reset"
                        )
                        button(
                            size: ButtonSize::Sm,
                            attrs: attributes! {
                                type="button"
                                :disabled=$(completed.get() >= 100)
                                @click=$(|_e: Event| {
                                    let next = completed.get() + 10;
                                    completed.set(if next > 100 { 100 } else { next });
                                })
                            },
                            "Advance"
                        )
                    </div>
                </div>
                separator(attrs: attributes! { class="my-4" })
                <div class="flex flex-col gap-2">
                    <p class="text-sm text-muted-foreground">"Indeterminate"</p>
                    progress(attrs: attributes! { aria-label="Indeterminate progress" })
                </div>
            )
        )
    })
}

/// The form controls, each with the label naming it.
///
/// The fields keep their values in signals. Reset restores their initial values.
#[component]
async fn form_card(cx: &Cx) -> Result<impl View> {
    let name = signal(cx, String::new);
    let region = signal(cx, || String::from("eu-central-1"));
    let summary = signal(cx, String::new);

    Ok(view! {
        card(
            card_header(
                card_title("Form controls")
                card_description("An input, a select, a textarea, and a label each.")
            )
            card_content(
                <form
                    class="flex flex-col gap-4"
                    @submit=$(|e: Event| e.prevent_default())
                    @reset=$(|e: Event| {
                        e.prevent_default();
                        name.set("".to_owned());
                        region.set("eu-central-1".to_owned());
                        summary.set("".to_owned());
                    })
                >
                    <div class="flex flex-col gap-2">
                        label(attrs: attributes! { for="project-name" }, "Name")
                        input(
                            attrs: attributes! {
                                id="project-name"
                                placeholder="my-app"
                                :value=$(name.get())
                                @input=$(|e: Event| name.set(e.target.value))
                            }
                        )
                    </div>
                    <div class="flex flex-col gap-2">
                        label(attrs: attributes! { for="region" }, "Region")
                        select(
                            attrs: attributes! {
                                id="region"
                                :value=$(region.get())
                                @change=$(|e: Event| region.set(e.target.value))
                            },
                            <optgroup label="Europe">
                                <legend>"Europe"</legend>
                                <option>"eu-central-1"</option>
                                <option>"eu-west-2"</option>
                            </optgroup>
                            <optgroup label="Americas">
                                <legend>"Americas"</legend>
                                <option>"us-east-1"</option>
                                <option>"sa-east-1"</option>
                            </optgroup>
                        )
                    </div>
                    <div class="flex flex-col gap-2">
                        label(attrs: attributes! { for="summary" }, "Summary")
                        textarea(
                            attrs: attributes! {
                                id="summary"
                                placeholder="What this project is for."
                                :value=$(summary.get())
                                @input=$(|e: Event| summary.set(e.target.value))
                            }
                        )
                    </div>
                    <div class="flex flex-col gap-2">
                        label(attrs: attributes! { for="owner" }, "Owner")
                        // A disabled field shows a value that is not the
                        // form's to change.
                        input(
                            attrs: attributes! { id="owner" value="ada@example.com" disabled="" }
                        )
                    </div>
                    <div class="flex flex-wrap justify-end gap-2">
                        button(
                            variant: ButtonVariant::Outline,
                            attrs: attributes! { type="reset" },
                            "Reset"
                        )
                    </div>
                </form>
            )
        )
    })
}

/// The states a checkbox is shown in: the id it goes by, the word for the
/// state, whether it is checked, and whether it is disabled.
const CHECKS: [(&str, &str, bool, bool); 4] = [
    ("check-on", "Checked", true, false),
    ("check-off", "Unchecked", false, false),
    ("check-on-off", "Checked and disabled", true, true),
    ("check-off-off", "Unchecked and disabled", false, true),
];

/// The same states, shown on a switch.
const SWITCHES: [(&str, &str, bool, bool); 3] = [
    ("switch-on", "On", true, false),
    ("switch-off", "Off", false, false),
    ("switch-off-off", "Off and disabled", false, true),
];

/// Checkboxes in their checked, unchecked, and disabled states.
///
/// Each control keeps its own signal, starting in the state its row names.
#[component]
async fn checks_card(cx: &Cx) -> Result<impl View> {
    let checks: Vec<_> = CHECKS
        .into_iter()
        .map(|(id, text, checked, disabled)| {
            (id, text, signal(&cx.keyed(id), || checked), disabled)
        })
        .collect();

    Ok(view! {
        card(
            card_header(
                card_title("Checkboxes")
                card_description("Checked, unchecked, and disabled.")
            )
            card_content(
                <div class="flex flex-col gap-3">
                    for (id, text, checked, disabled) in checks {
                        <div class="flex items-center gap-2">
                            checkbox(
                                attrs: attributes! {
                                    id=(id)
                                    :checked=$(checked.get())
                                    @change=$(|e: Event| checked.set(e.target.checked))
                                    disabled=(disabled)
                                }
                            )
                            label(
                                attrs: attributes! { for=(id) class=(class!("opacity-50" if disabled)) },
                                (text)
                            )
                        </div>
                    }
                </div>
            )
        )
    })
}

/// Switches with an independent signal for each control.
#[component]
async fn switches_card(cx: &Cx) -> Result<impl View> {
    let switches: Vec<_> = SWITCHES
        .into_iter()
        .map(|(id, text, checked, disabled)| {
            (id, text, signal(&cx.keyed(id), || checked), disabled)
        })
        .collect();

    Ok(view! {
        card(
            card_header(
                card_title("Switches")
                card_description("On, off, and disabled.")
            )
            card_content(
                <div class="flex flex-col gap-3">
                    for (id, text, checked, disabled) in switches {
                        <div class="flex items-center justify-between gap-4">
                            label(
                                attrs: attributes! { for=(id) class=(class!("opacity-50" if disabled)) },
                                (text)
                            )
                            switch(
                                attrs: attributes! {
                                    id=(id)
                                    :checked=$(checked.get())
                                    @change=$(|e: Event| checked.set(e.target.checked))
                                    disabled=(disabled)
                                }
                            )
                        </div>
                    }
                </div>
            )
        )
    })
}

/// A standalone radio group whose selection is managed by the browser.
#[component]
async fn radios_card() -> Result<impl View> {
    Ok(view! {
        card(
            card_header(
                card_title("Radio group")
                card_description("One choice at a time, with a disabled option.")
            )
            card_content(
                radio_group(
                    attrs: attributes! { aria-label="Example options" },
                    for (value, text, checked, disabled) in [
                        ("one", "Option one", true, false),
                        ("two", "Option two", false, false),
                        ("three", "Option three (disabled)", false, true),
                    ] {
                        let id = format!("radio-demo-{value}");

                        <div class="flex items-center gap-2">
                            radio_group_item(
                                attrs: attributes! {
                                    id=(id.as_str())
                                    name="radio-demo"
                                    value=(value)
                                    checked=(checked)
                                    disabled=(disabled)
                                }
                            )
                            label(attrs: attributes! { for=(id.as_str()) }, (text))
                        </div>
                    }
                )
            )
        )
    })
}

/// A card that switches panels in the browser.
#[component]
async fn overview_card(cx: &Cx) -> Result<impl View> {
    let selected = signal(cx, || TABS[0].0.to_owned());

    Ok(view! {
        card(
            card_header(
                card_title("Tabs")
                card_description("Switch between panels.")
            )
            card_content(
                tabs(
                    tabs_list(
                        for (value, text) in TABS {
                            tabs_trigger(
                                active: $(selected.get() == value),
                                attrs: attributes! {
                                    href="#"
                                    @click=$(|e: Event| {
                                        e.prevent_default();
                                        selected.set(value.to_owned());
                                    })
                                },
                                (text)
                            )
                        }
                    )
                    for (value, _) in TABS {
                        tabs_content(
                            attrs: attributes! { :hidden=$(selected.get() != value) },
                            <p class="text-sm text-muted-foreground">
                                (match value {
                                    "activity" => "Recent activity appears here.",
                                    "settings" => "Adjust your preferences here.",
                                    _ => "A quick overview of your project.",
                                })
                            </p>
                        )
                    }
                )
            )
        )
    })
}

/// A FAQ whose answers fold away, one open at a time.
#[component]
async fn faq_card() -> Result<impl View> {
    Ok(view! {
        card(
            card_header(
                card_title("Accordion")
                card_description("One section open at a time; the rest fold away.")
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
    })
}

/// A branch switcher that updates its label and closes the menu locally.
#[component]
async fn branches_card(cx: &Cx) -> Result<impl View> {
    let selected = signal(cx, || BRANCHES[0].to_owned());
    let open = signal(cx, || false);
    let tags_open = signal(cx, || false);

    Ok(view! {
        card(
            card_header(
                card_title("Dropdown menu")
                card_description("Pick a branch or tag to update the selection.")
            )
            card_content(
                dropdown_menu(
                    attrs: attributes! { :open=$(open.get()) },
                    // The trigger takes any content; this one borrows the
                    // outline button's looks and adds a flipping chevron.
                    dropdown_menu_trigger(
                        attrs: attributes! {
                            class=(button_variants(
                                ButtonVariant::Outline,
                                ButtonSize::Sm,
                            ))
                            @click=$(|e: Event| {
                                e.prevent_default();
                                open.toggle();
                                tags_open.set(false);
                            })
                        },
                        $(selected.get())
                        icon(
                            data: iconify_icon!("lucide:chevron-down"),
                            attrs: attributes! { class="transition-transform group-open:rotate-180" }
                        )
                    )
                    dropdown_menu_content(
                        dropdown_menu_label("Switch branch")
                        for branch in BRANCHES {
                            dropdown_menu_item(
                                attrs: attributes! {
                                    type="button"
                                    @click=$(|_e: Event| {
                                        selected.set(branch.to_owned());
                                        open.set(false);
                                        tags_open.set(false);
                                    })
                                },
                                (branch)
                            )
                        }
                        dropdown_menu_separator()
                        // A submenu opens its own panel beside this row.
                        dropdown_menu_sub(
                            attrs: attributes! { :open=$(tags_open.get()) },
                            dropdown_menu_sub_trigger(
                                attrs: attributes! {
                                    @click=$(|e: Event| {
                                        e.prevent_default();
                                        tags_open.toggle();
                                    })
                                },
                                "Checkout tag"
                            )
                            dropdown_menu_sub_content(
                                for tag in TAGS {
                                    dropdown_menu_item(
                                        attrs: attributes! {
                                            type="button"
                                            @click=$(|_e: Event| {
                                                selected.set(tag.to_owned());
                                                open.set(false);
                                                tags_open.set(false);
                                            })
                                        },
                                        (tag)
                                    )
                                }
                            )
                        )
                    )
                )
            )
        )
    })
}

/// A toolbar of toggles: a segmented control where picking one lets go of the
/// rest, and toggles that press on their own.
#[component]
async fn toolbar_card(cx: &Cx) -> Result<impl View> {
    let range = signal(cx, || String::from("week"));
    let bold = signal(cx, || true);
    let italic = signal(cx, || false);
    let underline = signal(cx, || false);
    let live = signal(cx, || true);

    Ok(view! {
        card(
            card_header(
                card_title("Toggles")
                card_description(
                    "A segmented control that keeps one pressed, and toggles \
                     that press on their own."
                )
            )
            card_content(
                <div class="flex flex-col items-start gap-4">
                    // The groups of a toolbar stand apart with a rule
                    // between them, and the row's height is what gives the
                    // rule its own.
                    <div class="flex h-9 items-center gap-2">
                        toggle_group(
                            for (value, text) in [
                                ("day", "Day"),
                                ("week", "Week"),
                                ("month", "Month"),
                            ] {
                                toggle(
                                    kind: ToggleKind::Exclusive,
                                    size: ToggleSize::Sm,
                                    attrs: attributes! {
                                        name="range"
                                        value=(value)
                                        :checked=$(range.get() == value)
                                        @change=$(|_e: Event| range.set(value.to_owned()))
                                    },
                                    (text)
                                )
                            }
                        )
                        separator(orientation: SeparatorOrientation::Vertical)
                        <div class="flex items-center gap-1">
                            for (name, data, text, pressed) in [
                                ("bold", iconify_icon!("lucide:bold"), "Bold", &bold),
                                (
                                    "italic",
                                    iconify_icon!("lucide:italic"),
                                    "Italic",
                                    &italic,
                                ),
                                (
                                    "underline",
                                    iconify_icon!("lucide:underline"),
                                    "Underline",
                                    &underline,
                                ),
                            ] {
                                toggle(
                                    attrs: attributes! {
                                        name=(name)
                                        :checked=$(pressed.get())
                                        @change=$(|e: Event| pressed.set(e.target.checked))
                                    },
                                    icon(data: data, label: text)
                                )
                            }
                        </div>
                    </div>
                    toggle(
                        size: ToggleSize::Md,
                        attrs: attributes! {
                            name="live"
                            :checked=$(live.get())
                            @change=$(|e: Event| live.set(e.target.checked))
                        },
                        icon(data: iconify_icon!("lucide:activity"))
                        "Live updates"
                    )
                </div>
            )
        )
    })
}

/// A short hint shown on hover or focus.
#[component]
async fn tooltip_card() -> Result<impl View> {
    Ok(view! {
        card(
            card_header(
                card_title("Tooltip")
                card_description("A short hint on hover or focus.")
            )
            card_content(
                <div class="flex items-center justify-between gap-4">
                    <p class="text-sm text-muted-foreground">"Hover the button"</p>
                    tooltip(
                        <a
                            href=(DOCS)
                            class=(button_variants(
                                ButtonVariant::Outline,
                                ButtonSize::Icon,
                            ))
                        >
                            icon(
                                data: iconify_icon!("lucide:book-open"),
                                label: "Read the docs"
                            )
                        </a>
                        tooltip_content("Read the docs")
                    )
                </div>
            )
        )
    })
}

/// A preview with richer content, shown on hover or focus.
#[component]
async fn hover_card_demo() -> Result<impl View> {
    Ok(view! {
        card(
            card_header(
                card_title("Hover card")
                card_description("A preview on hover or focus.")
            )
            card_content(
                <div class="flex items-center gap-2 text-sm">
                    <p class="text-muted-foreground">"Hover the name"</p>
                    hover_card(
                        // The trigger takes focus, so the card comes up for a
                        // reader on the keyboard as well.
                        <button type="button" class="font-medium underline">
                            "@ada"
                        </button>
                        hover_card_content(
                            <div class="flex items-center gap-3">
                                avatar(
                                    size: AvatarSize::Md,
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
                            <p class="text-sm text-muted-foreground">
                                "A hover card holds a view, where a tooltip \
                                 holds a few words."
                            </p>
                        )
                    )
                </div>
            )
        )
    })
}

/// A dialog and an alert dialog, each controlled by a local signal.
#[component]
async fn dialogs_card(cx: &Cx) -> Result<impl View> {
    let open = signal(cx, || false);
    let confirming = signal(cx, || false);

    Ok(view! {
        card(
            card_header(
                card_title("Dialog")
                card_description("A content panel or a confirmation prompt.")
            )
            card_content(
                <div class="flex flex-wrap gap-2">
                    button(
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        attrs: attributes! { type="button" @click=$(|_e: Event| open.set(true)) },
                        "Open dialog"
                    )
                    button(
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        attrs: attributes! { type="button" @click=$(|_e: Event| confirming.set(true)) },
                        "Open alert dialog"
                    )
                </div>
            )
        )
        dialog(
            open: $(open.get()),
            attrs: attributes! { aria-label="Example dialog" },
            dialog_content(
                dialog_header(
                    dialog_title("Example dialog")
                    dialog_description(
                        "A dialog brings content into focus above the page."
                    )
                )
                <p class="text-sm">
                    "Put your content here, then close the dialog to return to the page."
                </p>
                dialog_footer(
                    button(
                        attrs: attributes! { type="button" @click=$(|_e: Event| open.set(false)) },
                        "Close"
                    )
                )
            )
        )
        alert_dialog(
            open: $(confirming.get()),
            attrs: attributes! { aria-label="Continue?" },
            dialog_content(
                dialog_header(
                    dialog_title("Continue?")
                    dialog_description(
                        "An alert dialog asks for an explicit choice before continuing."
                    )
                )
                dialog_footer(
                    button(
                        variant: ButtonVariant::Outline,
                        attrs: attributes! {
                            type="button"
                            @click=$(|_e: Event| confirming.set(false))
                        },
                        "Cancel"
                    )
                    button(
                        attrs: attributes! {
                            type="button"
                            @click=$(|_e: Event| confirming.set(false))
                        },
                        "Continue"
                    )
                )
            )
        )
    })
}

/// A sheet controlled by a local signal.
#[component]
async fn sheet_card(cx: &Cx) -> Result<impl View> {
    let open = signal(cx, || false);

    Ok(view! {
        card(
            card_header(
                card_title("Sheet")
                card_description("A panel that slides in from the edge of the page.")
            )
            card_content(
                button(
                    variant: ButtonVariant::Outline,
                    attrs: attributes! { type="button" @click=$(|_e: Event| open.set(true)) },
                    "Open sheet"
                )
            )
        )
        sheet(
            open: $(open.get()),
            attrs: attributes! { aria-label="Example sheet" },
            sheet_content(
                dialog_header(
                    dialog_title("Example sheet")
                    dialog_description(
                        "Use a sheet for content that belongs beside the page."
                    )
                )
                <p class="text-sm">
                    "The rest of the page stays visible behind this panel."
                </p>
                dialog_footer(
                    attrs: attributes! { class="mt-auto" },
                    button(
                        attrs: attributes! { type="button" @click=$(|_e: Event| open.set(false)) },
                        "Close"
                    )
                )
            )
        )
    })
}

/// The deployments the table pages through: the commit, the environment it
/// went to, and the status it is in.
const DEPLOYMENTS: [(&str, &str, &str); 12] = [
    ("a1b2c3d", "production", "Live"),
    ("9f8e7d6", "staging", "Building"),
    ("4c5b6a7", "preview", "Queued"),
    ("2e1d0c9", "preview", "Failed"),
    ("7b6a5f4", "production", "Live"),
    ("3d2c1b0", "staging", "Live"),
    ("8e7d6c5", "preview", "Queued"),
    ("1a0b9c8", "production", "Failed"),
    ("5c4b3a2", "staging", "Building"),
    ("0f9e8d7", "preview", "Live"),
    ("6a5b4c3", "production", "Queued"),
    ("d4c3b2a", "staging", "Failed"),
];

/// A table of deployments with pagination underneath.
#[shard]
async fn deployments_card(cx: &Cx) -> Result<impl View> {
    let page = signal(cx, || 1usize);
    let rows = &DEPLOYMENTS;
    let pages = rows.len().div_ceil(PER_PAGE).max(1);
    // Clamp the requested page to the available rows.
    let current = page.get().clamp(1, pages);
    let previous = current.saturating_sub(1).max(1);
    let next = (current + 1).min(pages);

    Ok(view! {
        let shown = rows.chunks(PER_PAGE).nth(current - 1).unwrap_or_default();

        card(
            card_header(
                card_title("Table")
                card_description("Deployment rows with badges and pagination.")
            )
            // The card pads its sections rather than itself, so the table can
            // span its full width; the table's own padding lines the cells up
            // with the sections above and below it.
            table(
                attrs: attributes! { class="px-3" },
                table_caption("Deployments")
                table_header(
                    table_row(
                        table_head("Commit")
                        table_head("Environment")
                        table_head("Status")
                    )
                )
                table_body(
                    for (commit, env, status) in shown.iter().copied() {
                        table_row(
                            table_cell(
                                attrs: attributes! { class="font-mono" },
                                (commit)
                            )
                            table_cell((env))
                            table_cell(badge(variant: status_variant(status), (status)))
                        )
                    }
                )
                table_footer(
                    table_row(
                        table_cell(attrs: attributes! { colspan="2" }, "Total")
                        table_cell((format!("{} deployments", rows.len())))
                    )
                )
            )
            card_footer(
                attrs: attributes! { class="justify-center" },
                pagination(
                    pagination_content(
                        pagination_item(
                            pagination_previous(
                                attrs: attributes! {
                                    href="#"
                                    @click=$(|e: Event| {
                                        e.prevent_default();
                                        page.set(previous);
                                    })
                                }
                            )
                        )
                        for number in 1..=pages {
                            if listed(number, current, pages) {
                                pagination_item(
                                    pagination_link(
                                        active: number == current,
                                        attrs: attributes! {
                                            href="#"
                                            @click=$(|e: Event| {
                                                e.prevent_default();
                                                page.set(number);
                                            })
                                        },
                                        (number)
                                    )
                                )
                            } else if listed(number - 1, current, pages) {
                                // The first page left out of a run stands for
                                // the whole run.
                                pagination_item(pagination_ellipsis())
                            }
                        }
                        pagination_item(
                            pagination_next(
                                attrs: attributes! {
                                    href="#"
                                    @click=$(|e: Event| {
                                        e.prevent_default();
                                        page.set(next);
                                    })
                                }
                            )
                        )
                    )
                )
            )
        )
    })
}

/// Whether `number` gets a link of its own while `page` is the one being read:
/// the first page, the last one, and the current one do, and the runs left
/// between them collapse into an ellipsis.
///
/// Listing the current page's neighbours too, as a roomier pagination would,
/// grows the row past the width of a card in this masonry. The pagination
/// wraps rather than overflowing when that happens, but stepping one page at a
/// time is what "Previous" and "Next" are already for.
fn listed(number: usize, page: usize, pages: usize) -> bool {
    number == 1 || number == pages || number == page
}

/// The keys that move through a form, and what each one does. They are the
/// browser's own, so they work on this page as they read here.
const KEYS: [(&str, &[&str]); 3] = [
    ("Move to the next control", &["Tab"]),
    ("Move back to the one before", &["Shift", "Tab"]),
    ("Submit the form", &["Enter"]),
];

/// A breadcrumb trail with a collapsed middle section.
#[component]
async fn breadcrumbs_card() -> Result<impl View> {
    Ok(view! {
        card(
            card_header(
                card_title("Breadcrumbs")
                card_description("A trail of links to the current page.")
            )
            card_content(
                breadcrumb(
                    breadcrumb_list(
                        breadcrumb_item(
                            breadcrumb_link(attrs: attributes! { href=(DOCS) }, "Docs")
                        )
                        breadcrumb_separator()
                        // The steps between are collapsed into an ellipsis.
                        breadcrumb_item(breadcrumb_ellipsis())
                        breadcrumb_separator()
                        breadcrumb_item(
                            breadcrumb_link(
                                attrs: attributes! { href=(REGISTRY) },
                                "Components"
                            )
                        )
                        breadcrumb_separator()
                        breadcrumb_item(breadcrumb_page("Breadcrumbs"))
                    )
                )
            )
        )
    })
}

/// Individual keys and key combinations.
#[component]
async fn keyboard_card() -> Result<impl View> {
    Ok(view! {
        card(
            card_header(
                card_title("Keyboard keys")
                card_description("Keys shown individually or in a group.")
            )
            card_content(
                <div class="flex flex-col gap-3">
                    for (action, keys) in KEYS {
                        <div class="flex items-center justify-between gap-4">
                            <p class="text-sm text-muted-foreground">(action)</p>
                            kbd_group(
                                for key in keys.iter().copied() {
                                    kbd((key))
                                }
                            )
                        </div>
                    }
                </div>
            )
        )
    })
}

/// Skeleton placeholders that can be replaced with loaded content.
#[component]
async fn skeletons_card(cx: &Cx) -> Result<impl View> {
    let loading = signal(cx, || true);

    Ok(view! {
        card(
            card_header(
                card_title("Skeletons")
                card_description("Placeholders while content loads.")
            )
            card_content(
                // The skeletons take the size of what they stand in for, so
                // the card keeps its height once the roster lands.
                <div class="flex flex-col gap-4" :hidden=$(!loading.get())>
                    for _ in 0..2 {
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
                <div class="flex flex-col gap-4" :hidden=$(loading.get())>
                    for (name, email, initials, role) in MEMBERS.into_iter().take(2) {
                        <div class="flex items-center gap-3">
                            avatar(size: AvatarSize::Sm, avatar_fallback((initials)))
                            <div class="flex min-w-0 flex-1 flex-col gap-1.5">
                                <p class="truncate text-sm font-medium">(name)</p>
                                <p class="truncate text-xs text-muted-foreground">
                                    (email)
                                </p>
                            </div>
                            badge(variant: BadgeVariant::Outline, (role))
                        </div>
                    }
                </div>
            )
            card_footer(
                button(
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    attrs: attributes! { type="button" @click=$(|_e: Event| loading.toggle()) },
                    $(if loading.get() { "Show content" } else { "Show skeletons" })
                )
            )
        )
    })
}

/// A spinner accompanying a loading message.
#[component]
async fn spinner_card(cx: &Cx) -> Result<impl View> {
    let loading = signal(cx, || true);

    Ok(view! {
        card(
            card_header(
                card_title("Spinner")
                card_description("An indicator while work is in progress.")
            )
            card_content(
                <div class="flex flex-col gap-2">
                    <p class="flex items-center gap-1.5 text-sm text-muted-foreground">
                        <span class="contents" :hidden=$(!loading.get())>
                            spinner()
                        </span>
                        $(if loading.get() { "Loading" } else { "Complete" })
                    </p>
                    button(
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        attrs: attributes! {
                            type="button"
                            class="self-end"
                            @click=$(|_e: Event| loading.toggle())
                        },
                        $(if loading.get() { "Finish loading" } else { "Load again" })
                    )
                </div>
            )
        )
    })
}

/// The showcase navigation, with independent desktop and mobile controls.
#[component]
async fn app_sidebar(
    cx: &Cx,
    open: &Signal<bool>,
    mobile_open: &Signal<bool>,
) -> Result<impl View> {
    let selected = signal(cx, || "#overview".to_owned());
    let sections = signal(cx, || true);

    Ok(view! {
        sidebar(
            open: $(open.get()),
            mobile_open: $(mobile_open.get()),
            collapsible: SidebarCollapsible::Icon,
            sheet_attrs: attributes! {
                id="showcase-sidebar"
                aria-label="Component library navigation"
                @keydown=$(|e: Event| {
                    if e.key == "Escape" {
                        mobile_open.set(false);
                    }
                })
                @click=$(|e: Event| {
                    if e.target.id == "showcase-sidebar" {
                        mobile_open.set(false);
                    }
                })
            },
            sidebar_header(
                <div class="flex items-center gap-1">
                    sidebar_menu(
                        attrs: attributes! { class="flex-1" },
                        sidebar_menu_item(
                            sidebar_menu_button(
                                size: SidebarMenuButtonSize::Lg,
                                href: Some("#overview"),
                                tooltip: Some("Topcoat UI"),
                                attrs: attributes! {
                                    @click=$(|_e: Event| {
                                        selected.set("#overview".to_owned());
                                        mobile_open.set(false);
                                    })
                                },
                                icon(data: iconify_icon!("lucide:layers"))
                                <span class="flex min-w-0 flex-col text-left">
                                    <span class="font-semibold">"Topcoat UI"</span>
                                    <span class="text-xs text-muted-foreground">
                                        "Your component library"
                                    </span>
                                </span>
                            )
                        )
                    )
                    button(
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Icon,
                        attrs: attributes! {
                            type="button"
                            class="md:hidden"
                            aria-label="Close sidebar"
                            @click=$(|_e: Event| mobile_open.set(false))
                        },
                        icon(data: iconify_icon!("lucide:x"))
                    )
                </div>
            )
            sidebar_content(
                sidebar_group(
                    sidebar_group_label("Workspace")
                    sidebar_group_action(
                        attrs: attributes! {
                            aria-label="Toggle section links"
                            aria-controls="sidebar-section-links"
                            :aria-expanded=$(if sections.get() {
                                "true"
                            } else {
                                "false"
                            })
                            @click=$(|_e: Event| sections.toggle())
                        },
                        icon(data: iconify_icon!("lucide:chevrons-up-down"))
                    )
                    sidebar_group_content(
                        sidebar_menu(
                            sidebar_menu_item(
                                sidebar_menu_button(
                                    href: Some("#overview"),
                                    tooltip: Some("Overview"),
                                    active: $(selected.get() == "#overview"),
                                    attrs: attributes! {
                                        @click=$(|_e: Event| {
                                            selected.set("#overview".to_owned());
                                            mobile_open.set(false);
                                        })
                                    },
                                    icon(data: iconify_icon!("lucide:house"))
                                    <span>"Overview"</span>
                                )
                            )
                            sidebar_menu_item(
                                sidebar_menu_button(
                                    href: Some("#components"),
                                    tooltip: Some("Components"),
                                    active: $(selected.get() == "#components"),
                                    attrs: attributes! {
                                        @click=$(|_e: Event| {
                                            selected.set("#components".to_owned());
                                            mobile_open.set(false);
                                        })
                                    },
                                    icon(data: iconify_icon!("lucide:layout-grid"))
                                    <span>"Components"</span>
                                )
                                sidebar_menu_badge("22")
                                sidebar_menu_sub(
                                    attrs: attributes! { id="sidebar-section-links" :hidden=$(!sections.get()) },
                                    for (href, text) in [
                                        ("#forms", "Forms"),
                                        ("#overlays", "Overlays"),
                                    ] {
                                        sidebar_menu_sub_item(
                                            sidebar_menu_sub_button(
                                                active: $(selected.get() == href),
                                                attrs: attributes! {
                                                    href=(href)
                                                    @click=$(|_e: Event| {
                                                        selected.set(href.to_owned());
                                                        mobile_open.set(false);
                                                    })
                                                },
                                                <span>(text)</span>
                                            )
                                        )
                                    }
                                )
                            )
                        )
                    )
                )
                sidebar_group(
                    sidebar_group_label("Resources")
                    sidebar_group_content(
                        sidebar_menu(
                            for (href, text, data) in [
                                (DOCS, "Documentation", iconify_icon!("lucide:book-open")),
                                (REGISTRY, "Registry", iconify_icon!("lucide:blocks")),
                                (REPOSITORY, "Source code", iconify_icon!("lucide:github")),
                            ] {
                                sidebar_menu_item(
                                    sidebar_menu_button(
                                        href: Some(href),
                                        tooltip: Some(text),
                                        icon(data: data)
                                        <span>(text)</span>
                                    )
                                )
                            }
                        )
                    )
                )
            )
            sidebar_separator()
            sidebar_footer(
                <p
                    class="px-2 py-1 text-xs text-muted-foreground md:group-data-[collapsible=icon]/sidebar:hidden"
                >
                    "Built with Topcoat. Yours to make your own."
                </p>
            )
        )
    })
}
