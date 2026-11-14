use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::{error::RouterErrorExt, href, module_param, page},
    runtime::{Signal, signal},
    view::{StaticClass, View, class, component, view},
};

use super::ListingLink;
use crate::{
    app::{self, products},
    cart::{CART_MAX, cart_signal},
    components::{
        breadcrumbs::{breadcrumbs, crumb},
        error_page::PageError,
        icons::{self, icon},
        palette::Palette,
        product_card::product_card,
        product_image::product_image,
        stars::{StarSize, filled_stars, stars},
    },
    data::{Product, store},
    format::{Count, Money, Rating, iso_date, short_date},
};

module_param!(pub product_id: u32);

const STEPPER_BUTTON: StaticClass = class!(
    "rounded-full p-2.5 text-stone-700 hover:bg-stone-100 disabled:cursor-not-allowed disabled:opacity-30 disabled:hover:bg-transparent"
);

/// The product named by the request's path.
fn current_product(cx: &Cx) -> Option<&Product> {
    let id = product_id(cx).ok()?;
    store(cx).product(*id)
}

/// `GET /products/{id}`: product details, specifications, reviews and related products.
#[page(
    with(Metadata::from_fn(
        async |cx| {
            let title = match current_product(cx) {
                Some(product) => format!("{} | Harbor Market", product.name),
                None => PageError::NotFound.document_title().to_owned(),
            };
            Ok(Metadata::new().title(title))
        },
    ))
)]
pub async fn detail(cx: &Cx) -> Result<impl View> {
    let store = store(cx);
    let product = current_product(cx).ok_or_not_found()?;
    let category = store.category_of(product);
    let category_link = || href!(products::listing).query(ListingLink::category(&category.slug));
    let rating = Rating(product.rating).to_string();
    // The cart badge in the header reads the same signal.
    let cart = cart_signal(cx);

    Ok(view! {
        <div class="mx-auto max-w-7xl px-4 py-10 sm:px-6 lg:px-8">
            breadcrumbs(
                current: &product.name,
                crumb(href: href!(app::home), label: "Home")
                crumb(href: href!(products::listing), label: "Products")
                crumb(href: category_link(), label: &category.name)
            )
            <div class="mt-8 grid gap-10 lg:grid-cols-2 lg:gap-16">
                <div class="lg:sticky lg:top-24 lg:self-start">
                    product_image(product: product)
                </div>
                <div>
                    <a
                        href=(category_link())
                        class="text-sm font-semibold text-teal-700 hover:text-teal-900"
                    >
                        (&category.name)
                    </a>
                    <h1
                        class="mt-2 text-3xl font-semibold tracking-tight text-balance text-stone-900 sm:text-4xl"
                    >
                        (&product.name)
                    </h1>
                    <div class="mt-3 flex items-center gap-2">
                        stars(
                            filled: filled_stars(product.rating_tenths()),
                            rating: &rating,
                            size: StarSize::Large
                        )
                        <span class="text-sm font-medium text-stone-900">
                            (&rating)
                        </span>
                        <a
                            href="#reviews"
                            class="text-sm text-stone-500 underline-offset-4 hover:text-stone-900 hover:underline"
                        >
                            (Count(product.review_count.into()).to_string())
                            " reviews"
                        </a>
                    </div>
                    <div class="mt-6 flex flex-wrap items-baseline gap-3">
                        <p class="text-3xl font-semibold tracking-tight text-stone-900">
                            (Money(product.price_cents.into()).to_string())
                        </p>
                        if let Some(compare_at) = product.compare_at_cents {
                            let saving = compare_at - product.price_cents;
                            <s class="text-lg text-stone-400">
                                (Money(compare_at.into()).to_string())
                            </s>
                            <span
                                class="rounded-full bg-rose-50 px-2.5 py-1 text-sm font-medium text-rose-700"
                            >
                                "Save "
                                (Money(saving.into()).to_string())
                                " ("
                                (saving * 100 / compare_at)
                                "%)"
                            </span>
                        }
                    </div>
                    <p class="mt-4 text-lg leading-8 text-stone-600">
                        (&product.tagline)
                    </p>
                    <dl class="mt-6 grid grid-cols-[auto_1fr] gap-x-8 gap-y-3 text-sm">
                        <dt class="font-medium text-stone-900">"Color"</dt>
                        <dd class="flex items-center gap-2 text-stone-600">
                            <span
                                class=(class!(
                                    "size-4 rounded-full ring-1 ring-black/10",
                                    Palette::of(product.color).accent,
                                ))
                            ></span>
                            (product.color.label())
                        </dd>
                        <dt class="font-medium text-stone-900">"Availability"</dt>
                        match product.stock {
                            0 => <dd class="font-medium text-stone-500">"Sold out"</dd>,
                            stock @ 1..10 => <dd class="font-medium text-amber-700">
                                "Only "
                                (stock)
                                " left in stock"
                            </dd>,
                            _ => <dd class="font-medium text-emerald-700">
                                "In stock"
                            </dd>,
                        }
                    </dl>
                    if product.is_sold_out() {
                        <div class="mt-8">
                            <button
                                type="button"
                                disabled=(true)
                                class="w-full cursor-not-allowed rounded-full bg-stone-200 px-8 py-3.5 text-base font-semibold text-stone-500"
                            >
                                "Sold out"
                            </button>
                        </div>
                    } else {
                        purchase_form(max: product.max_quantity(), cart: &cart)
                    }
                    <ul
                        class="mt-8 space-y-3 border-t border-stone-200 pt-6 text-sm text-stone-600"
                    >
                        <li class="flex items-center gap-3">
                            icon(icon: icons::CHECK)
                            "Free shipping on orders over $75"
                        </li>
                        <li class="flex items-center gap-3">
                            icon(icon: icons::CHECK)
                            "Free 30-day returns"
                        </li>
                        <li class="flex items-center gap-3">
                            icon(icon: icons::CHECK)
                            "Ships within one business day"
                        </li>
                    </ul>
                </div>
            </div>
            <div
                class="mt-16 grid gap-12 border-t border-stone-200 pt-16 lg:grid-cols-2"
            >
                <section>
                    <h2 class="text-xl font-semibold tracking-tight">"Description"</h2>
                    <div class="mt-4 space-y-4 leading-7 text-stone-600">
                        for paragraph in product.paragraphs() {
                            <p>(paragraph)</p>
                        }
                    </div>
                </section>
                <section>
                    <h2 class="text-xl font-semibold tracking-tight">
                        "Specifications"
                    </h2>
                    <table class="mt-4 w-full text-left text-sm">
                        <tbody
                            class="divide-y divide-stone-200 border-y border-stone-200"
                        >
                            for spec in &product.specs {
                                <tr>
                                    <th
                                        scope="row"
                                        class="w-2/5 py-3 pr-4 font-medium text-stone-900"
                                    >
                                        (&spec.label)
                                    </th>
                                    <td class="py-3 text-stone-600">(&spec.value)</td>
                                </tr>
                            }
                        </tbody>
                    </table>
                </section>
            </div>
            <section
                id="reviews"
                class="mt-16 grid gap-10 border-t border-stone-200 pt-16 lg:grid-cols-[18rem_1fr]"
            >
                <div>
                    <h2 class="text-xl font-semibold tracking-tight">
                        "Customer reviews"
                    </h2>
                    <div class="mt-4 flex items-center gap-3">
                        <p class="text-4xl font-semibold tracking-tight">(&rating)</p>
                        stars(
                            filled: filled_stars(product.rating_tenths()),
                            rating: &rating,
                            size: StarSize::Large
                        )
                    </div>
                    <p class="mt-2 text-sm text-stone-600">
                        "Based on "
                        (Count(product.review_count.into()).to_string())
                        " reviews"
                    </p>
                    <p class="mt-1 text-sm text-stone-500">
                        "Showing the "
                        (product.reviews.len())
                        " most recent."
                    </p>
                </div>
                <ul class="divide-y divide-stone-200">
                    #[key(index)]
                    for (index, review) in product.reviews.iter().enumerate() {
                        <li class="py-6 first:pt-0">
                            <article>
                                <div class="flex items-center justify-between gap-4">
                                    stars(
                                        filled: review.rating,
                                        rating: review.rating,
                                        size: StarSize::Small
                                    )
                                    <time
                                        datetime=(iso_date(review.date))
                                        class="text-sm text-stone-500"
                                    >
                                        (short_date(review.date))
                                    </time>
                                </div>
                                <h3 class="mt-3 font-semibold text-stone-900">
                                    (&review.title)
                                </h3>
                                <p class="mt-2 leading-7 text-stone-600">(&review.body)</p>
                                <p class="mt-3 text-sm font-medium text-stone-900">
                                    (&review.author)
                                </p>
                            </article>
                        </li>
                    }
                </ul>
            </section>
            <section
                aria-labelledby="related-heading"
                class="mt-16 border-t border-stone-200 pt-16"
            >
                <h2 id="related-heading" class="text-xl font-semibold tracking-tight">
                    "You might also like"
                </h2>
                <div class="mt-8 grid grid-cols-2 gap-x-6 gap-y-10 lg:grid-cols-4">
                    #[key(related.id)]
                    for related in store.related(product) {
                        product_card(product: related)
                    }
                </div>
            </section>
        </div>
    })
}

/// The quantity stepper and add-to-cart button of an in-stock product.
///
/// Runs in the browser: the stepper keeps the quantity within `1..=max`, and adding to the cart
/// updates the header badge and the `cart_count` cookie.
#[component]
async fn purchase_form(cx: &Cx, max: u32, cart: &Signal<u32>) -> Result<impl View> {
    let quantity = signal(cx, || 1u32);
    // The quantity of the last add to cart, or zero before the first one.
    let added = signal(cx, || 0u32);
    // The browser code writes the cart cookie as `{cookie}{count}{attributes}`.
    let (cookie, attributes) = ("cart_count=", "; Path=/; Max-Age=2592000; SameSite=Lax");
    Ok(view! {
        <div class="mt-8">
            <div class="flex flex-col gap-3 sm:flex-row">
                <div
                    class="flex items-center justify-between rounded-full border border-stone-300 bg-white p-1 sm:w-36"
                >
                    <button
                        type="button"
                        aria-label="Decrease quantity"
                        :disabled=$(quantity.get() <= 1u32)
                        @click=$(|_e| {
                            if quantity.get() > 1u32 {
                                quantity.decrement()
                            }
                        })
                        class=(STEPPER_BUTTON)
                    >
                        icon(icon: icons::MINUS)
                    </button>
                    <output
                        aria-live="polite"
                        aria-label="Quantity"
                        class="w-10 text-center font-semibold tabular-nums"
                    >
                        $(quantity.get())
                    </output>
                    <button
                        type="button"
                        aria-label="Increase quantity"
                        :disabled=$(quantity.get() >= max)
                        @click=$(|_e| {
                            if quantity.get() < max {
                                quantity.increment()
                            }
                        })
                        class=(STEPPER_BUTTON)
                    >
                        icon(icon: icons::PLUS)
                    </button>
                </div>
                <button
                    type="button"
                    @click=$(|_e| {
                        let total = cart.get() + quantity.get();
                        let total = if total > CART_MAX { CART_MAX } else { total };
                        cart.set(total);
                        added.set(quantity.get());
                        raw!("document.cookie = ${cookie} + ${total} + ${attributes}");
                    })
                    class="flex-1 rounded-full bg-teal-700 px-8 py-3.5 text-base font-semibold text-white shadow-sm hover:bg-teal-800 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-teal-700"
                >
                    "Add to cart"
                </button>
            </div>
            <p role="status" class="mt-3 min-h-5 text-sm font-medium text-teal-800">
                $(if added.get() > 0u32 {
                    Some(("Added ", added.get(), " to your cart."))
                } else {
                    None
                })
            </p>
        </div>
    })
}
