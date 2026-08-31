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
    sheet::{SheetSide, sheet, sheet_content},
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
    router::{Router, RouterBuilderDiscoverExt, page, query_params},
    tailwind,
    view::{View, attributes, class, component, view},
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
        .assets(AssetBundle::load().unwrap())
        .discover()
        .build();

    topcoat::start(router).await.unwrap();
}

/// What the page keeps in its URL.
///
/// Every key is optional, so a query string that makes no sense reloads the
/// page without one rather than failing the request.
#[query_params(error = redirect("?"))]
struct HomeQuery {
    tab: Option<String>,
    page: Option<usize>,
    per_page: Option<usize>,
    name: Option<String>,
    env: Option<String>,
    status: Option<String>,
    branch: Option<String>,
    overlay: Option<String>,
    side: Option<String>,
}

/// The page's state: its query string read back as the values the page has
/// markup for.
///
/// Everything the page does without scripting goes through here. A link or a
/// form sets one part of the state, the page comes back rendered for it, and
/// the state survives a reload and can be shared as it stands.
struct State {
    /// The panel the project card shows.
    tab: &'static str,
    /// The page of the deployments table.
    page: usize,
    /// How many rows a page of the deployments table holds.
    per_page: usize,
    /// The name the project goes by.
    name: String,
    /// The environment the deployments table is filtered to.
    env: Option<&'static str>,
    /// The status the deployments table is filtered to.
    status: Option<&'static str>,
    /// The branch the preview builds from.
    branch: &'static str,
    /// The overlay covering the page.
    overlay: Option<&'static str>,
    /// The edge the sheet comes in from.
    side: &'static str,
}

impl State {
    /// Reads the state out of the request's query string.
    ///
    /// Every value is looked up among the ones the page has markup for, so a
    /// query string nobody wrote cannot put the page in a state it has no way
    /// to render.
    fn read(cx: &Cx) -> Result<Self> {
        let query = query_params::<HomeQuery>(cx)?;
        let branch = query.branch.as_deref();

        Ok(Self {
            tab: one_of(query.tab.as_deref(), &TABS.map(|(value, _)| value)).unwrap_or(TABS[0].0),
            page: query.page.unwrap_or(1).max(1),
            per_page: one_of_numbers(query.per_page, &PER_PAGE).unwrap_or(PER_PAGE[0]),
            name: project_name(query.name.clone()),
            env: one_of(query.env.as_deref(), &ENVIRONMENTS),
            status: query.status.as_deref().and_then(status_label),
            branch: one_of(branch, &BRANCHES)
                .or(one_of(branch, &TAGS))
                .unwrap_or(BRANCHES[0]),
            overlay: one_of(query.overlay.as_deref(), &OVERLAYS.map(|(value, _)| value)),
            side: one_of(query.side.as_deref(), &SIDES.map(|(value, ..)| value))
                .unwrap_or(SIDES[0].0),
        })
    }

    /// The state as query parameters, leaving out whatever stands at its
    /// default: a link to the page as it is carries no query string at all.
    fn params(&self) -> Vec<(&'static str, String)> {
        let mut params = Vec::new();

        if self.tab != TABS[0].0 {
            params.push(("tab", self.tab.to_owned()));
        }
        if self.page > 1 {
            params.push(("page", self.page.to_string()));
        }
        if self.per_page != PER_PAGE[0] {
            params.push(("per_page", self.per_page.to_string()));
        }
        if self.name != NAME {
            params.push(("name", self.name.clone()));
        }
        if let Some(env) = self.env {
            params.push(("env", env.to_owned()));
        }
        if let Some(status) = self.status {
            params.push(("status", status.to_lowercase()));
        }
        if self.branch != BRANCHES[0] {
            params.push(("branch", self.branch.to_owned()));
        }
        if let Some(overlay) = self.overlay {
            params.push(("overlay", overlay.to_owned()));
        }
        if self.side != SIDES[0].0 {
            params.push(("side", self.side.to_owned()));
        }

        params
    }

    /// The page's URL with `key` set to `value`, or dropped for `None`, and
    /// the rest of the state left as it is.
    ///
    /// Every link on the page is built this way, which is what keeps opening a
    /// dialog or turning a page from resetting the parts of the page it has
    /// nothing to do with.
    fn href(&self, key: &'static str, value: Option<&str>) -> String {
        let mut params = self.params();
        params.retain(|(name, _)| *name != key);
        if let Some(value) = value {
            params.push((key, value.to_owned()));
        }

        let query: Vec<String> = params
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect();

        format!("?{}", query.join("&"))
    }

    /// The URL of page `number` of the deployments table.
    fn page_href(&self, number: usize) -> String {
        if number > 1 {
            self.href("page", Some(&number.to_string()))
        } else {
            self.href("page", None)
        }
    }

    /// The URL of the sheet lying against `side`.
    fn side_href(&self, side: &'static str) -> String {
        self.href("side", (side != SIDES[0].0).then_some(side))
    }

    /// The URL of the page with no overlay over it, which is what closes one.
    fn closed(&self) -> String {
        self.href("overlay", None)
    }

    /// Whether a deployment to `env` in `status` passes the table's filters.
    fn shows(&self, env: &str, status: &str) -> bool {
        self.env.is_none_or(|filter| filter == env)
            && self.status.is_none_or(|filter| filter == status)
    }

    /// What the deployments table is filtered to, in words.
    fn filtered(&self) -> String {
        match (self.env, self.status) {
            (None, None) => String::from("Every environment"),
            (Some(env), None) => format!("Deployments to {env}"),
            (None, Some(status)) => format!("{status} deployments"),
            (Some(env), Some(status)) => format!("{status} deployments to {env}"),
        }
    }

    /// The edge the sheet lies against.
    fn sheet_side(&self) -> SheetSide {
        SIDES
            .iter()
            .find(|(value, ..)| *value == self.side)
            .map_or(SheetSide::default(), |(.., side)| *side)
    }
}

/// The entry of `values` that `value` names, if it names one at all.
fn one_of(value: Option<&str>, values: &[&'static str]) -> Option<&'static str> {
    let value = value?;
    values.iter().copied().find(|known| *known == value)
}

/// The entry of `values` that `value` names, for the numbers among the state.
fn one_of_numbers(value: Option<usize>, values: &[usize]) -> Option<usize> {
    let value = value?;
    values.iter().copied().find(|known| *known == value)
}

/// The name the project goes by until it is renamed.
const NAME: &str = "topcoat-ui";

/// The name in `value`, if it is one the page's links can carry.
///
/// The links here build their query strings by hand, so a name is kept to
/// what needs no escaping: letters, digits, dashes, and underscores, and no
/// more than 32 of them. Anything else falls back to the default, the same
/// way the rest of the state does.
fn project_name(value: Option<String>) -> String {
    let name = value.unwrap_or_default();
    let carried = !name.is_empty()
        && name.len() <= 32
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

    if carried { name } else { NAME.to_owned() }
}

/// The panels the project card tabs between: the value each goes by in the
/// URL, and the word on its trigger. The first is the one the page opens on.
const TABS: [(&str, &str); 3] = [
    ("overview", "Overview"),
    ("activity", "Activity"),
    ("settings", "Settings"),
];

/// How many rows one page of the deployments table can hold. The first is the
/// number it holds until another is picked.
const PER_PAGE: [usize; 3] = [3, 6, 12];

/// The environments deployments go to.
const ENVIRONMENTS: [&str; 3] = ["production", "staging", "preview"];

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

/// The overlays that can cover the page, one at a time: the value each goes
/// by in the URL, and the word for it.
const OVERLAYS: [(&str, &str); 3] = [
    ("rename", "Dialog"),
    ("reset", "Alert dialog"),
    ("filters", "Sheet"),
];

/// The edges the sheet can come in from: the value each goes by in the URL,
/// the word for it, and the side itself. The first is the one it comes from
/// until another is picked.
const SIDES: [(&str, &str, SheetSide); 4] = [
    ("right", "Right", SheetSide::Right),
    ("left", "Left", SheetSide::Left),
    ("top", "Top", SheetSide::Top),
    ("bottom", "Bottom", SheetSide::Bottom),
];

/// The word for the deployment status `value` names, whatever its case.
fn status_label(value: &str) -> Option<&'static str> {
    STATUSES
        .iter()
        .find(|(status, _)| status.eq_ignore_ascii_case(value))
        .map(|(status, _)| *status)
}

/// The badge variant the deployment status `status` shows in.
fn status_variant(status: &str) -> BadgeVariant {
    STATUSES
        .iter()
        .find(|(known, _)| *known == status)
        .map_or(BadgeVariant::default(), |(_, variant)| *variant)
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    let state = State::read(cx)?;

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

                    // A masonry of small, self-contained demos, each built
                    // from the installed components. They run from the plainest
                    // components to the ones assembled out of them.
                    <div class="mt-14 columns-1 gap-4 sm:columns-2 xl:columns-3">
                        demo(buttons_card())
                        demo(notices())
                        demo(team_card())
                        demo(status_card())
                        demo(form_card())
                        demo(checks_card())
                        demo(rows_card(state: &state))
                        demo(overlays_card(state: &state))
                        demo(overview_card(state: &state))
                        demo(faq_card())
                        demo(branches_card(state: &state))
                        demo(toolbar_card())
                        demo(share_card())
                        demo(rename_card(state: &state))
                        demo(deployments_card(state: &state))
                        demo(docs_card())
                        demo(pending_card())
                        demo(deploy_card())
                    </div>
                </main>

                // The overlays cover the page, so they stand outside the
                // masonry rather than in the cells that open them.
                rename_dialog(state: &state)
                reset_dialog(state: &state)
                filters_sheet(state: &state)
            </body>
        </html>
    }
}

/// A masonry cell: keeps a demo from splitting across columns.
#[component]
async fn demo(child: View) -> Result {
    view! { <div class="mb-4 break-inside-avoid">(child)</div> }
}

/// The parts of the page's state a form does not set, carried along as hidden
/// fields.
///
/// A form submitted with GET replaces the whole query string with its own
/// fields, so without these, submitting one would reset the rest of the page.
/// `sets` names the parameters the form has controls for, separated by spaces;
/// those are left out, since the form submits its own values for them.
#[component]
async fn state_fields(state: &State, sets: &str) -> Result {
    view! {
        for (key, value) in state.params() {
            if !sets.split_whitespace().any(|set| set == key) {
                <input type="hidden" name=(key) value=(value)>
            }
        }
    }
}

/// The button family: variants, sizes, and states at a glance.
#[component]
async fn buttons_card() -> Result {
    view! {
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
    }
}

/// Notices standing on their own: an alert is a surface already, so it needs
/// no card under it.
#[component]
async fn notices() -> Result {
    view! {
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
    }
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
async fn team_card() -> Result {
    view! {
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
    }
}

/// Environment statuses told through the badge variants, and a rollout told
/// through the progress bar.
#[component]
async fn status_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Badges")
                card_description(
                    "Every variant, counting the rows of the table below, and \
                     a progress bar under them."
                )
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
                separator(attrs: attributes! { class="my-4" })
                <div class="flex flex-col gap-2">
                    <div class="flex items-center justify-between gap-4">
                        <p class="text-sm text-muted-foreground">
                            "A bar with a value"
                        </p>
                        <p class="text-sm font-medium">"62%"</p>
                    </div>
                    progress(value: 62.0)
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
    }
}

/// The form controls, each with the label naming it.
///
/// Nothing is submitted here: the fields are the demo, and "Reset" is the one
/// control that acts, which the browser does on its own.
#[component]
async fn form_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Form controls")
                card_description("An input, a select, a textarea, and a label each.")
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
                            <optgroup label="Europe">
                                <option>"eu-central-1"</option>
                                <option>"eu-west-2"</option>
                            </optgroup>
                            <optgroup label="Americas">
                                <option>"us-east-1"</option>
                                <option>"sa-east-1"</option>
                            </optgroup>
                        )
                    </div>
                    <div class="flex flex-col gap-2">
                        label(attrs: attributes! { for="summary" }, "Summary")
                        textarea(
                            attrs: attributes! { id="summary" placeholder="What this project is for." }
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
    }
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

/// The checkbox and the switch, in each of the states they can be in.
///
/// Nothing behind them keeps a value, so every row says which state it stands
/// in rather than naming a setting the page does not have.
#[component]
async fn checks_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Checkboxes and switches")
                card_description("Each one in the states it can be in.")
            )
            card_content(
                <div class="flex flex-col gap-3">
                    for (id, text, checked, disabled) in CHECKS {
                        <div class="flex items-center gap-2">
                            checkbox(
                                attrs: attributes! { id=(id) checked=(checked) disabled=(disabled) }
                            )
                            label(
                                attrs: attributes! { for=(id) class=(class!("opacity-50" if disabled)) },
                                (text)
                            )
                        </div>
                    }
                </div>
                separator(attrs: attributes! { class="my-4" })
                <div class="flex flex-col gap-3">
                    for (id, text, checked, disabled) in SWITCHES {
                        <div class="flex items-center justify-between gap-4">
                            label(
                                attrs: attributes! { for=(id) class=(class!("opacity-50" if disabled)) },
                                (text)
                            )
                            switch(
                                attrs: attributes! { id=(id) checked=(checked) disabled=(disabled) }
                            )
                        </div>
                    }
                </div>
            )
        )
    }
}

/// A radio group that sets how many rows the table further down shows.
///
/// The choice reaches the server the way every other one on this page does:
/// the group sits in a form, and the button submits it into the URL. The page
/// is left out of the fields carried along, so a table resized by hand comes
/// back at its first page rather than at one the rows no longer reach.
#[component]
async fn rows_card(state: &State) -> Result {
    view! {
        card(
            card_header(
                card_title("Radio group")
                card_description(
                    "One choice at a time. This one sets how many rows the \
                     table below shows."
                )
            )
            card_content(
                <form class="flex flex-col gap-4">
                    state_fields(state: state, sets: "per_page page")
                    radio_group(
                        // The name the options share is what has the browser
                        // let go of one when another is picked.
                        for rows in PER_PAGE {
                            let id = format!("rows-{rows}");

                            <div class="flex items-center gap-2">
                                radio_group_item(
                                    attrs: attributes! {
                                        id=(id.as_str())
                                        name="per_page"
                                        value=(rows.to_string())
                                        checked=(rows == state.per_page)
                                    }
                                )
                                label(
                                    attrs: attributes! { for=(id.as_str()) },
                                    (format!("{rows} rows a page"))
                                )
                            </div>
                        }
                    )
                    button(
                        size: ButtonSize::Sm,
                        attrs: attributes! { class="self-start" },
                        "Apply"
                    )
                </form>
            )
        )
    }
}

/// The overlays gathered in one place, so each can be opened without hunting
/// for the card it belongs to.
///
/// A trigger is a plain link to this page with the overlay named in its query
/// string, which is all it takes to open one: the same overlays are opened
/// from the cards they belong to further down.
#[component]
async fn overlays_card(state: &State) -> Result {
    view! {
        card(
            card_header(
                card_title("Overlays")
                card_description(
                    "The URL is what holds one open, so it survives a reload \
                     and can be linked to."
                )
            )
            card_content(
                <div class="flex flex-wrap gap-2">
                    for (overlay, name) in OVERLAYS {
                        <a
                            href=(state.href("overlay", Some(overlay)))
                            class=(button_variants(
                                ButtonVariant::Outline,
                                ButtonSize::Sm,
                            ))
                        >
                            "Open "
                            (name.to_lowercase())
                        </a>
                    }
                </div>
            )
        )
    }
}

/// A card that tabs between panels.
///
/// Which panel shows is in the URL, so each trigger is a link and only the
/// panel being read is rendered.
#[component]
async fn overview_card(state: &State) -> Result {
    view! {
        card(
            card_header(
                card_title("Tabs")
                card_description("The panel being read is the one named in the URL.")
            )
            card_content(
                tabs(
                    tabs_list(
                        for (value, text) in TABS {
                            tabs_trigger(
                                active: value == state.tab,
                                attrs: attributes! { href=(state.href("tab", Some(value))) },
                                (text)
                            )
                        }
                    )
                    tabs_content(
                        <p class="text-sm text-muted-foreground">
                            (match state.tab {
                                "activity" => {
                                    "Reload the page and this same panel comes back."
                                }
                                "settings" => "Send the URL on and it opens here too.",
                                _ => "This panel is in the URL as ?tab=overview.",
                            })
                        </p>
                    )
                )
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
    }
}

/// A branch switcher: a menu whose items reach the server, since a menu item
/// is a button and a form around it is all it takes.
#[component]
async fn branches_card(state: &State) -> Result {
    view! {
        card(
            card_header(
                card_title("Dropdown menu")
                card_description(
                    "Picking an item submits the form around it, so the choice \
                     lands in the URL."
                )
            )
            card_content(
                // The form carries the rest of the page's state along, and the
                // item that was clicked adds the branch it stands for; the
                // page comes back built from that branch.
                <form>
                    state_fields(state: state, sets: "branch")
                    dropdown_menu(
                        // The trigger takes any content; this one borrows the
                        // outline button's looks and adds a flipping chevron.
                        dropdown_menu_trigger(
                            attrs: attributes! {
                                class=(button_variants(
                                    ButtonVariant::Outline,
                                    ButtonSize::Sm,
                                ))
                            },
                            (state.branch)
                            icon(
                                data: iconify_icon!("lucide:chevron-down"),
                                attrs: attributes! { class="transition-transform group-open:rotate-180" }
                            )
                        )
                        dropdown_menu_content(
                            dropdown_menu_label("Switch branch")
                            for branch in BRANCHES {
                                dropdown_menu_item(
                                    attrs: attributes! { name="branch" value=(branch) },
                                    (branch)
                                )
                            }
                            dropdown_menu_separator()
                            // A submenu opens its own panel beside this row.
                            dropdown_menu_sub(
                                dropdown_menu_sub_trigger("Checkout tag")
                                dropdown_menu_sub_content(
                                    for tag in TAGS {
                                        dropdown_menu_item(
                                            attrs: attributes! { name="branch" value=(tag) },
                                            (tag)
                                        )
                                    }
                                )
                            )
                        )
                    )
                </form>
            )
        )
    }
}

/// A toolbar of toggles: a segmented control where picking one lets go of the
/// rest, and toggles that press on their own.
#[component]
async fn toolbar_card() -> Result {
    view! {
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
                        separator(orientation: SeparatorOrientation::Vertical)
                        <div class="flex items-center gap-1">
                            for (name, data, text, pressed) in [
                                ("bold", iconify_icon!("lucide:bold"), "Bold", true),
                                ("italic", iconify_icon!("lucide:italic"), "Italic", false),
                                (
                                    "underline",
                                    iconify_icon!("lucide:underline"),
                                    "Underline",
                                    false,
                                ),
                            ] {
                                toggle(
                                    attrs: attributes! { name=(name) checked=(pressed) },
                                    icon(data: data, label: text)
                                )
                            }
                        </div>
                    </div>
                    toggle(
                        size: ToggleSize::Lg,
                        attrs: attributes! { name="live" checked="" },
                        icon(data: iconify_icon!("lucide:activity"))
                        "Live updates"
                    )
                </div>
            )
        )
    }
}

/// The two things that show on hover: a tooltip carrying a few words, and a
/// hover card carrying a view.
///
/// Both triggers are the browser's own hover and focus, so nothing here needs
/// scripting. The tooltip's trigger is a link that goes where the hint says,
/// since a hint is only a hint.
#[component]
async fn share_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Tooltip and hover card")
                card_description(
                    "What comes up on hover: a few words, or a whole view."
                )
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
                separator(attrs: attributes! { class="my-4" })
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
                            <p class="text-xs text-muted-foreground">
                                "A hover card holds a view, where a tooltip \
                                 holds a few words."
                            </p>
                        )
                    )
                </div>
            )
        )
    }
}

/// The name the page carries, and the two overlays that act on it.
///
/// Nothing about a trigger is special: opening an overlay is navigating to the
/// URL the page renders it open for.
#[component]
async fn rename_card(state: &State) -> Result {
    view! {
        card(
            card_header(
                card_title("Project name")
                card_description(
                    "The name is part of the URL, and the dialog changes it."
                )
            )
            card_content(
                <div class="flex items-center justify-between gap-4">
                    <p class="truncate font-mono text-sm">(&state.name)</p>
                    <a
                        href=(state.href("overlay", Some("rename")))
                        class=(button_variants(ButtonVariant::Outline, ButtonSize::Sm))
                    >
                        "Rename"
                    </a>
                </div>
                separator(attrs: attributes! { class="my-4" })
                <div class="flex items-center justify-between gap-4">
                    <p class="truncate text-sm text-muted-foreground">
                        "Put the whole page back"
                    </p>
                    // Clearing the page goes through an alert dialog, so it
                    // takes a deliberate answer rather than one stray click.
                    <a
                        href=(state.href("overlay", Some("reset")))
                        class=(button_variants(
                            ButtonVariant::Destructive,
                            ButtonSize::Sm,
                        ))
                    >
                        "Reset"
                    </a>
                </div>
            )
        )
    }
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

/// A table of deployments, filtered by the sheet, sized by the radio group,
/// and paginated underneath.
///
/// Every one of those comes from the URL, so the links below the table are
/// what change the rows and which page reads as the current one.
#[component]
async fn deployments_card(state: &State) -> Result {
    let rows: Vec<_> = DEPLOYMENTS
        .into_iter()
        .filter(|&(_, env, status)| state.shows(env, status))
        .collect();
    let pages = rows.len().div_ceil(state.per_page).max(1);
    // A filter can leave fewer pages than the URL asks for, so the page being
    // read is the last one that still has rows on it.
    let page = state.page.min(pages);
    let shown = rows
        .chunks(state.per_page)
        .nth(page - 1)
        .unwrap_or_default();
    let previous = state.page_href(page.saturating_sub(1).max(1));
    let next = state.page_href((page + 1).min(pages));

    view! {
        card(
            card_header(
                card_title("Table")
                card_description(
                    "Filtered from the sheet, sized by the radio group, and \
                     paged by the links below."
                )
            )
            card_content(
                <div class="flex items-center justify-between gap-4">
                    <p class="truncate text-sm text-muted-foreground">
                        (state.filtered())
                    </p>
                    // What the sheet holds would not fit a dialog, so it comes
                    // in from the edge instead.
                    <a
                        href=(state.href("overlay", Some("filters")))
                        class=(button_variants(ButtonVariant::Outline, ButtonSize::Sm))
                    >
                        icon(data: iconify_icon!("lucide:filter"))
                        "Filters"
                    </a>
                </div>
            )
            // The card pads its sections rather than itself, so the table can
            // span its full width; the table's own padding lines the cells up
            // with the sections above and below it.
            table(
                attrs: attributes! { class="px-3" },
                table_caption(
                    "Stand-in rows, so the table has something to page through."
                )
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
                            pagination_previous(attrs: attributes! { href=(previous) })
                        )
                        for number in 1..=pages {
                            if listed(number, page, pages) {
                                pagination_item(
                                    pagination_link(
                                        active: number == page,
                                        attrs: attributes! { href=(state.page_href(number)) },
                                        (number)
                                    )
                                )
                            } else if listed(number - 1, page, pages) {
                                // The first page left out of a run stands for
                                // the whole run.
                                pagination_item(pagination_ellipsis())
                            }
                        }
                        pagination_item(
                            pagination_next(attrs: attributes! { href=(next) })
                        )
                    )
                )
            )
        )
    }
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

/// A documentation page header: the trail to it, and the keys it documents.
#[component]
async fn docs_card() -> Result {
    view! {
        card(
            card_header(
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
                        breadcrumb_item(breadcrumb_page("Dialog"))
                    )
                )
                card_title("Dialog")
                card_description("A panel over the page for a single task.")
            )
            card_content(
                // The section a docs page ends on: how the dialog's form is
                // worked from the keyboard.
                <h3 class="text-sm font-medium">"Keyboard"</h3>
                <div class="mt-3 flex flex-col gap-3">
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
    }
}

/// The shapes a page takes while it waits: the roster before it arrives, and
/// the two ways of saying that work is under way.
#[component]
async fn pending_card() -> Result {
    view! {
        card(
            card_header(
                card_title("Skeletons and spinners")
                card_description("The shapes a page takes while it waits.")
            )
            card_content(
                // The skeletons take the size of what they stand in for, so
                // the card keeps its height once the roster lands.
                <div class="flex flex-col gap-4">
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
                separator(attrs: attributes! { class="my-4" })
                <div class="flex flex-col gap-2">
                    <p class="flex items-center gap-1.5 text-sm text-muted-foreground">
                        spinner()
                        "Work whose end is not in sight"
                    </p>
                    // Without a value the bar reads as work whose extent is
                    // not known yet.
                    progress()
                </div>
            )
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
                    card_title("Dark scheme")
                    card_description(
                        "The same components, on a wrapper that carries the \
                         `dark` class."
                    )
                )
                card_footer(
                    button(size: ButtonSize::Sm, "Primary")
                    button(size: ButtonSize::Sm, variant: ButtonVariant::Ghost, "Ghost")
                )
            )
        </div>
    }
}

/// The dialog over the page, shown while the URL names it.
///
/// Closing it is navigating back to the page that renders it closed, which the
/// corner button and "Cancel" do as links. "Save" submits the form, which puts
/// the new name in the URL and leaves the overlay out of it, so the page comes
/// back renamed with the dialog closed.
#[component]
async fn rename_dialog(state: &State) -> Result {
    view! {
        dialog(
            open: state.overlay == Some("rename"),
            dialog_content(
                // The panel is positioned, so a close control can sit in one
                // of its corners.
                <a
                    href=(state.closed())
                    class=(class!(
                        button_variants(ButtonVariant::Ghost, ButtonSize::Icon),
                        "absolute top-3 right-3",
                    ))
                >
                    icon(data: iconify_icon!("lucide:x"), label: "Close")
                </a>
                <form class="flex flex-col gap-4">
                    state_fields(state: state, sets: "overlay name")
                    dialog_header(
                        dialog_title("Rename project")
                        dialog_description(
                            "The name is carried in the query string, so it \
                             survives a reload and travels with the link."
                        )
                    )
                    <div class="flex flex-col gap-2">
                        label(attrs: attributes! { for="rename" }, "Name")
                        // A name the links could not carry as it stands is put
                        // back to the default, which is what the pattern and
                        // the length say before the form is even sent.
                        input(
                            attrs: attributes! {
                                id="rename"
                                name="name"
                                value=(&state.name)
                                maxlength="32"
                                pattern="[A-Za-z0-9_-]+"
                            }
                        )
                    </div>
                    dialog_footer(
                        <a
                            href=(state.closed())
                            class=(button_variants(ButtonVariant::Ghost, ButtonSize::Md))
                        >
                            "Cancel"
                        </a>
                        button("Save")
                    )
                </form>
            )
        )
    }
}

/// The alert dialog behind the reset action: it asks the question and offers
/// nothing but the two answers to it.
///
/// Resetting is a link to the page without a query string, which is the page
/// with every choice on it back at its default.
#[component]
async fn reset_dialog(state: &State) -> Result {
    // Bound out here rather than inline: a hyphenated attribute name inside an
    // `attributes!` nested in a `view!` currently trips `topcoat fmt`.
    let labels = attributes! {
        aria-labelledby="reset-title"
        aria-describedby="reset-description"
    };

    view! {
        alert_dialog(
            open: state.overlay == Some("reset"),
            attrs: labels,
            dialog_content(
                dialog_header(
                    dialog_title(
                        attrs: attributes! { id="reset-title" },
                        "Reset this page?"
                    )
                    dialog_description(
                        attrs: attributes! { id="reset-description" },
                        "The panel, the filters, the page of the table, the \
                         branch, and the name all go back to their defaults."
                    )
                )
                dialog_footer(
                    <a
                        href=(state.closed())
                        class=(button_variants(ButtonVariant::Ghost, ButtonSize::Md))
                    >
                        "Leave it as it is"
                    </a>
                    <a
                        href="?"
                        class=(button_variants(
                            ButtonVariant::Destructive,
                            ButtonSize::Md,
                        ))
                    >
                        "Reset the page"
                    </a>
                )
            )
        )
    }
}

/// The sheet behind the deployments table's "Filters" link: a panel along one
/// edge, holding what a dialog would be too small for.
///
/// Applying the filters is submitting the form, which puts them in the URL and
/// leaves out the overlay, so the sheet closes on the filtered table.
#[component]
async fn filters_sheet(state: &State) -> Result {
    view! {
        sheet(
            open: state.overlay == Some("filters"),
            sheet_content(
                side: state.sheet_side(),
                dialog_header(
                    dialog_title("Filters")
                    dialog_description("Narrow the deployments in the table.")
                )
                // A sheet can lie against any edge. The row picks which,
                // through links, since a form would take its unsaved values
                // along.
                <div class="flex flex-col gap-2">
                    <p class="text-sm font-medium">"Side"</p>
                    tabs_list(
                        for (value, text, _) in SIDES {
                            tabs_trigger(
                                active: value == state.side,
                                attrs: attributes! { href=(state.side_href(value)) },
                                (text)
                            )
                        }
                    )
                </div>
                <form class="flex flex-1 flex-col gap-4">
                    state_fields(state: state, sets: "env status page overlay side")
                    <div class="flex flex-col gap-2">
                        label(attrs: attributes! { for="filter-env" }, "Environment")
                        select(
                            attrs: attributes! { id="filter-env" name="env" },
                            <option value="">"All environments"</option>
                            for env in ENVIRONMENTS {
                                <option value=(env) selected=(state.env == Some(env))>
                                    (env)
                                </option>
                            }
                        )
                    </div>
                    <div class="flex flex-col gap-2">
                        <p class="text-sm font-medium">"Status"</p>
                        radio_group(
                            <div class="flex items-center gap-2">
                                radio_group_item(
                                    attrs: attributes! {
                                        id="filter-any"
                                        name="status"
                                        value=""
                                        checked=(state.status.is_none())
                                    }
                                )
                                label(
                                    attrs: attributes! { for="filter-any" },
                                    "Any status"
                                )
                            </div>
                            for (status, _) in STATUSES {
                                let value = status.to_lowercase();
                                let id = format!("filter-{value}");

                                <div class="flex items-center gap-2">
                                    radio_group_item(
                                        attrs: attributes! {
                                            id=(id.as_str())
                                            name="status"
                                            value=(value.as_str())
                                            checked=(state.status == Some(status))
                                        }
                                    )
                                    label(attrs: attributes! { for=(id.as_str()) }, (status))
                                </div>
                            }
                        )
                    </div>
                    dialog_footer(
                        attrs: attributes! { class="mt-auto" },
                        <a
                            href=(state.closed())
                            class=(button_variants(ButtonVariant::Ghost, ButtonSize::Md))
                        >
                            "Cancel"
                        </a>
                        button("Apply filters")
                    )
                </form>
            )
        )
    }
}
