//! The pages and API routes. Module paths map to URLs.

pub mod about;
pub mod admin;
pub mod api;
mod health;
mod not_found;
pub mod products;

use topcoat::{
    Result,
    asset::{Asset, AssetBundle, RouterBuilderAssetExt, asset},
    context::Cx,
    cookie::CookieLayer,
    metadata::Metadata,
    router::{Compression, Router, Slot, href, layout, module_router, page},
    runtime::RuntimeLayer,
    view::{View, error_boundary, view},
};

use crate::{
    cart::cart_signal,
    components::{
        error_page::{PageError, error_page},
        footer::footer,
        header::header,
        icons::{self, icon},
        product_card::product_card,
    },
    data::{Store, store},
    format::Count,
};
use products::ListingLink;

/// The Tailwind stylesheet compiled by `build.rs`.
///
/// It is written to a fixed path rather than the default `OUT_DIR` used by
/// `topcoat::tailwind::stylesheet!()`, because an asset's id includes its path and `OUT_DIR`
/// changes with the enabled cargo features. `topcoat asset bundle` cannot select features, so the
/// bundle it builds has to match the binary built with an allocator feature.
const STYLESHEET: Asset = asset!("target/tailwind.css");

/// Builds the application's router.
pub fn router(store: Store, compression: bool) -> Router {
    let router = module_router!()
        .with(store)
        .assets(AssetBundle::load().expect("failed to load the asset bundle"))
        .layer(CookieLayer::new())
        // Lets the browser runtime rerun pages; register it after other layers.
        .layer(RuntimeLayer::new());
    // Topcoat compresses responses by default.
    if compression {
        router.build()
    } else {
        router.with(Compression::disable()).build()
    }
}

#[layout(with(Metadata::new().title("Harbor Market")))]
async fn root(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let cart = cart_signal(cx);
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                topcoat::metadata::tags()
                <link rel="icon" href="data:,">
                <link rel="stylesheet" href=(STYLESHEET)>
                topcoat::runtime::script()
                topcoat::dev::script()
            </head>
            <body
                class="flex min-h-screen flex-col bg-stone-50 text-stone-900 antialiased"
            >
                <a
                    href="#main"
                    class="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-white focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:shadow"
                >
                    "Skip to content"
                </a>
                header(cart: &cart)
                <main id="main" class="flex-1">
                    error_boundary(
                        fallback: |error| {
                            let Some(page_error) = PageError::of(&error) else {
                                return Err(error);
                            };
                            Ok(view! { error_page(error: page_error) })
                        },
                        (slot)
                    )
                </main>
                footer()
            </body>
        </html>
    })
}

/// `GET /`: hero, store perks, category links and featured products.
#[page]
pub async fn home(cx: &Cx) -> Result<impl View> {
    let store = store(cx);
    let perks = [
        (
            icons::TRUCK,
            "Free shipping over $75",
            "Orders ship within one business day from Portland.",
        ),
        (
            icons::REFRESH,
            "30-day returns",
            "Changed your mind? Send it back, on us.",
        ),
        (
            icons::SHIELD,
            "Repair, don't replace",
            "Spare parts and repairs for as long as we sell it.",
        ),
    ];
    Ok(view! {
        <section class="relative overflow-hidden bg-teal-900 text-white">
            <div
                class="mx-auto grid max-w-7xl items-center gap-12 px-4 py-20 sm:px-6 lg:grid-cols-2 lg:px-8 lg:py-28"
            >
                <div>
                    <p
                        class="text-sm font-semibold uppercase tracking-widest text-teal-200"
                    >
                        "New season, same harbor"
                    </p>
                    <h1
                        class="mt-4 text-4xl font-semibold tracking-tight text-balance sm:text-5xl lg:text-6xl"
                    >
                        "Well-made gear for every harbor day."
                    </h1>
                    <p class="mt-6 max-w-xl text-lg leading-8 text-teal-100">
                        "Tents, knives, boots and blankets from small workshops we know by name. Built to last, easy to repair and shipped free over $75."
                    </p>
                    <div class="mt-10 flex flex-wrap gap-4">
                        <a
                            href=(href!(products::listing))
                            class="rounded-full bg-white px-6 py-3 text-sm font-semibold text-teal-900 shadow-sm hover:bg-teal-50"
                        >
                            "Shop all products"
                        </a>
                        <a
                            href=(href!(products::listing).query(
                                ListingLink::category("outdoor"),
                            ))
                            class="rounded-full px-6 py-3 text-sm font-semibold text-white ring-1 ring-inset ring-white/40 hover:bg-white/10"
                        >
                            "Browse outdoor gear"
                        </a>
                    </div>
                </div>
                <div aria-hidden="true" class="relative hidden aspect-[4/3] lg:block">
                    <div
                        class="absolute left-[8%] top-[10%] aspect-square w-[46%] -rotate-6 rounded-[28%] bg-amber-200 shadow-2xl shadow-black/30"
                    ></div>
                    <div
                        class="absolute right-[6%] top-[4%] aspect-square w-[34%] rotate-12 rounded-full bg-sky-400 shadow-2xl shadow-black/30"
                    ></div>
                    <div
                        class="absolute bottom-[2%] right-[18%] aspect-square w-[40%] rotate-6 rounded-[28%] bg-orange-700 shadow-2xl shadow-black/30"
                    ></div>
                </div>
            </div>
        </section>
        <section
            aria-label="Why Harbor Market"
            class="border-b border-stone-200 bg-white"
        >
            <ul
                class="mx-auto grid max-w-7xl gap-8 px-4 py-10 sm:grid-cols-3 sm:px-6 lg:px-8"
            >
                for (perk_icon, title, text) in perks {
                    <li class="flex gap-4">
                        <span
                            class="flex size-11 shrink-0 items-center justify-center rounded-full bg-teal-50 text-teal-700"
                        >
                            icon(icon: perk_icon)
                        </span>
                        <div>
                            <p class="font-semibold text-stone-900">(title)</p>
                            <p class="mt-1 text-sm text-stone-600">(text)</p>
                        </div>
                    </li>
                }
            </ul>
        </section>
        <section
            aria-labelledby="categories-heading"
            class="mx-auto max-w-7xl px-4 pt-16 sm:px-6 lg:px-8"
        >
            <h2 id="categories-heading" class="text-2xl font-semibold tracking-tight">
                "Shop by category"
            </h2>
            <ul class="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
                for category in store.categories() {
                    <li>
                        <a
                            href=(href!(products::listing).query(
                                ListingLink::category(&category.slug),
                            ))
                            class="group block h-full rounded-2xl border border-stone-200 bg-white p-5 transition hover:border-teal-600 hover:shadow-md"
                        >
                            <h3
                                class="font-semibold text-stone-900 group-hover:text-teal-800"
                            >
                                (&category.name)
                            </h3>
                            <p class="mt-1 line-clamp-2 text-sm text-stone-600">
                                (&category.description)
                            </p>
                            <p class="mt-4 text-xs font-medium text-stone-500">
                                (Count(category.product_count as u64).to_string())
                                " products"
                            </p>
                        </a>
                    </li>
                }
            </ul>
        </section>
        <section
            aria-labelledby="featured-heading"
            class="mx-auto max-w-7xl px-4 pt-20 sm:px-6 lg:px-8"
        >
            <div class="flex items-end justify-between gap-4">
                <h2 id="featured-heading" class="text-2xl font-semibold tracking-tight">
                    "Featured products"
                </h2>
                <a
                    href=(href!(products::listing))
                    class="flex items-center gap-1.5 text-sm font-semibold text-teal-700 hover:text-teal-900"
                >
                    "View all"
                    icon(icon: icons::ARROW_RIGHT)
                </a>
            </div>
            <div class="mt-8 grid grid-cols-2 gap-x-6 gap-y-10 lg:grid-cols-4">
                #[key(product.id)]
                for product in store.featured() {
                    product_card(product: product)
                }
            </div>
        </section>
    })
}
