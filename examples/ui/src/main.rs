mod components;

use components::button::{ButtonSize, ButtonVariant, button, button_variants};
use components::card::{
    card, card_content, card_description, card_footer, card_header, card_title,
};
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    font::fontsource::fontsource_font,
    icon::{icon, iconify},
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
                topcoat::font::link(font: fontsource_font!(LEXEND, host: Asset))
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
            </head>
            // The body's background, text color, and font come from the
            // theme's base layer in styles.css; nothing to set up here.
            <body>
                <main class="mx-auto max-w-3xl px-6 py-16">
                    <h1 class="text-4xl font-bold tracking-tight">"Topcoat UI"</h1>
                    <p class="mt-3 max-w-xl text-muted-foreground">
                        "Components vendored into this project with "
                        <code class="text-foreground">"topcoat ui add"</code>
                        ", styled by the design tokens of the installed theme."
                    </p>
                    <div class="mt-6 flex flex-wrap items-center gap-3">
                        button(
                            size: ButtonSize::Lg,
                            "Get started"
                            icon(data: iconify::iconify_icon!("feather:arrow-right"))
                        )
                        // Anything can borrow a button's looks: `button_variants`
                        // returns the class string for a variant and size.
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

                    showcase(
                        title: "Variants",
                        description: "Five variants rank actions from the one \
                            primary action of a screen down to inline commands, \
                            plus a destructive style for irreversible ones.",
                        for (variant, label) in [
                            (ButtonVariant::Primary, "Primary"),
                            (ButtonVariant::Secondary, "Secondary"),
                            (ButtonVariant::Outline, "Outline"),
                            (ButtonVariant::Ghost, "Ghost"),
                            (ButtonVariant::Destructive, "Destructive"),
                        ] {
                            button(variant: variant, (label))
                        }
                    )

                    showcase(
                        title: "Sizes",
                        description: "Three text sizes and a square one for \
                            icon-only buttons. Icons are 1em square, so they \
                            follow the button's text size.",
                        button(size: ButtonSize::Sm, "Small")
                        button(size: ButtonSize::Md, "Medium")
                        button(size: ButtonSize::Lg, "Large")
                        button(
                            size: ButtonSize::Icon,
                            variant: ButtonVariant::Outline,
                            icon(
                                data: iconify::iconify_icon!("feather:plus"),
                                label: "Add item"
                            )
                        )
                    )
                    showcase(
                        title: "Disabled",
                        description: "Extra attributes forward to the underlying \
                            <button> element, so disabling works like plain HTML.",
                        button(attrs: attributes! { disabled=(true) }, "Saving...")
                        button(
                            variant: ButtonVariant::Outline,
                            attrs: attributes! { disabled=(true) },
                            "Undo"
                        )
                    )

                    showcase(
                        title: "Card",
                        description: "A card stacks a header, content, and footer \
                            on a bordered, raised surface. Every section is \
                            optional.",
                        card(
                            attrs: attributes! { class="max-w-sm" },
                            card_header(
                                card_title("Invite your team")
                                card_description(
                                    "Collaborators get access to every project \
                                     in this workspace."
                                )
                            )
                            card_content(
                                <p class="text-sm">
                                    "Invited members can view and edit projects, \
                                     but only owners can change billing."
                                </p>
                            )
                            card_footer(
                                button(size: ButtonSize::Sm, "Send invites")
                                button(
                                    size: ButtonSize::Sm,
                                    variant: ButtonVariant::Ghost,
                                    "Copy link"
                                )
                            )
                        )
                    )

                    showcase(
                        title: "In context",
                        description: "A confirmation card pairing a quiet dismiss \
                            action with a destructive commit.",
                        card(
                            attrs: attributes! { class="max-w-sm" },
                            card_header(
                                card_title("Delete workspace")
                                card_description(
                                    "This permanently removes the workspace and \
                                     all of its data."
                                )
                            )
                            card_footer(
                                attrs: attributes! { class="justify-end" },
                                button(variant: ButtonVariant::Ghost, "Cancel")
                                button(
                                    variant: ButtonVariant::Destructive,
                                    "Delete workspace"
                                )
                            )
                        )
                    )

                    showcase(
                        title: "Color schemes",
                        description: "Components reference theme tokens instead of \
                            raw colors, so putting the dark class on an ancestor \
                            restyles everything inside it.",
                        <div class="grid w-full gap-4 sm:grid-cols-2">
                            scheme_panel(label: "Light")
                            <div class="dark">scheme_panel(label: "Dark")</div>
                        </div>
                    )
                </main>
            </body>
        </html>
    }
}

/// A titled gallery section with a wrapping row for its demos.
#[component]
async fn showcase(title: &'static str, description: &'static str, child: View) -> Result {
    view! {
        <section class="mt-14">
            <h2 class="text-lg font-semibold">(title)</h2>
            <p class="mt-1 max-w-xl text-sm text-muted-foreground">(description)</p>
            <div class="mt-5 flex flex-wrap items-center gap-3">(child)</div>
        </section>
    }
}

/// One color scheme's rendition of the button variants, on its own background.
#[component]
async fn scheme_panel(label: &'static str) -> Result {
    view! {
        card(
            card_content(
                <p class="text-xs font-medium text-muted-foreground">(label)</p>
                <div class="mt-3 flex flex-wrap items-center gap-2">
                    button(size: ButtonSize::Sm, "Primary")
                    button(
                        size: ButtonSize::Sm,
                        variant: ButtonVariant::Secondary,
                        "Secondary"
                    )
                    button(
                        size: ButtonSize::Sm,
                        variant: ButtonVariant::Outline,
                        "Outline"
                    )
                    button(size: ButtonSize::Sm, variant: ButtonVariant::Ghost, "Ghost")
                    button(
                        size: ButtonSize::Sm,
                        variant: ButtonVariant::Destructive,
                        "Destructive"
                    )
                </div>
            )
        )
    }
}
