mod menu;

use serde::Deserialize;
use toasty::Db;
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt, asset},
    context::Cx,
    cookie::RouterBuilderCookieExt,
    font::{Font, fontsource::fontsource_font},
    router::{
        Router, RouterBuilderDiscoverExt,
        content::Form,
        error::{SeeOther, see_other},
        layout, module_router, page, route,
    },
    tailwind,
    view::{attributes, view},
};

use crate::{
    components::{
        button::{ButtonSize, ButtonVariant, button, button_variants},
        card::{card, card_content, card_description, card_footer, card_header, card_title},
        input::input,
        label::label,
    },
    customer::{current_customer, forget_customer, remember_customer},
    drinks::drinks,
};

/// The theme's sans font, pulled from the Fontsource catalog and self-hosted
/// as a Topcoat asset.
const GEIST: Font = fontsource_font!(GEIST, host: Asset);

pub fn router(db: Db) -> Router {
    module_router!()
        // The font, the shard, and the procedure are collected at link time.
        .discover()
        .assets(AssetBundle::load().unwrap())
        .app_context(db)
        .cookies()
        .build()
}

// The layout in the root module wraps every page.
#[layout]
async fn shell(cx: &Cx, slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Little Crema"</title>
                topcoat::dev::script()

                // Signals, shards, and procedures need the browser runtime.
                topcoat::runtime::script()

                topcoat::font::link(font: GEIST)
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
            </head>
            <body class="flex min-h-screen flex-col">
                <header class="border-b border-border">
                    <nav
                        class="mx-auto flex w-full max-w-3xl items-center gap-6 px-6 py-4"
                    >
                        <a href="/" class="font-semibold">"Little Crema"</a>
                        <a
                            href="/menu"
                            class="text-sm text-muted-foreground hover:text-foreground"
                        >
                            "Menu"
                        </a>

                        match current_customer(cx) {
                            Some(name) => {
                                <p class="ml-auto text-sm text-muted-foreground">
                                    "Hi, "
                                    (name)
                                </p>
                            }
                            None => "",
                        }
                    </nav>
                </header>

                <main class="mx-auto w-full max-w-3xl flex-1 px-6 py-10">(slot?)</main>

                <footer class="border-t border-border">
                    <p
                        class="mx-auto w-full max-w-3xl px-6 py-4 text-sm text-muted-foreground"
                    >
                        // `drinks` is request-memoized, so this shares a menu
                        // query with pages that already loaded it.
                        (drinks(cx).await?.len())
                        " drinks brewed fresh daily."
                    </p>
                </footer>
            </body>
        </html>
    }
}

// A page in the root module renders at /.
#[page]
async fn home(cx: &Cx) -> Result {
    view! {
        <section class="flex flex-col items-center gap-5 pt-6 text-center">
            <img src=(asset!("./hero.svg")) alt="A cup of coffee" class="size-36">

            <h1 class="text-4xl font-bold tracking-tight">"Little Crema"</h1>

            <p class="max-w-md text-muted-foreground">
                "Six drinks, one tiny counter, and a Topcoat feature in every pour."
            </p>

            <a
                href="/menu"
                class=(button_variants(ButtonVariant::Primary, ButtonSize::Lg))
            >
                "Browse the menu"
            </a>
        </section>

        <section class="mx-auto mt-14 w-full max-w-sm">
            match current_customer(cx) {
                Some(name) => {
                    card(
                        card_header(
                            card_title(
                                "Welcome back, "
                                (&name)
                            )
                            card_description("Your usual table is free.")
                        )
                        card_footer(
                            // Submitting an empty name clears the cookie.
                            <form method="post" action="/">
                                button(
                                    variant: ButtonVariant::Outline,
                                    size: ButtonSize::Sm,
                                    attrs: attributes! { name="name" value="" },
                                    "Not "
                                    (name)
                                    "?"
                                )
                            </form>
                        )
                    )
                }
                None => {
                    card(
                        card_header(
                            card_title("Are you a regular?")
                            card_description(
                                "Leave your name and the counter remembers you."
                            )
                        )
                        card_content(
                            <form method="post" action="/" class="flex flex-col gap-3">
                                <div class="flex flex-col gap-2">
                                    label(attrs: attributes! { for="name" }, "Your name")
                                    input(
                                        attrs: attributes! { id="name" name="name" placeholder="Ada" required="" }
                                    )
                                </div>
                                button(attrs: attributes! { type="submit" }, "Remember me")
                            </form>
                        )
                    )
                }
            }
        </section>
    }
}

#[derive(Deserialize)]
struct SignIn {
    name: String,
}

// A route in the root module serves POST / for the forms above.
#[route(POST)]
async fn sign_in(cx: &Cx, Form(form): Form<SignIn>) -> Result<SeeOther> {
    match form.name.trim() {
        "" => forget_customer(cx),
        name => remember_customer(cx, name),
    }

    // Post/Redirect/Get, so a reload does not submit the form again.
    Ok(see_other("/"))
}
