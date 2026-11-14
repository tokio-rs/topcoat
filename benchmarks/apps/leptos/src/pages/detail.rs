use std::sync::Arc;

use leptos::{either::Either, prelude::*};
use leptos_router::hooks::use_params_map;
use serde::{Deserialize, Serialize};

use super::ErrorPage;
use crate::{
    cart::CartCount,
    components::{
        Breadcrumbs, PageTitle, Palette, ProductCard, ProductImage, StarSize, Stars, filled_stars,
        icons,
    },
    format::{Count, IsoDate, Money, Rating, ShortDate},
    model::{Category, PageError, Product, ProductSummary},
    urls::ListingUrl,
};

const STEPPER_BUTTON: &str = "rounded-full p-2.5 text-stone-700 hover:bg-stone-100 disabled:cursor-not-allowed disabled:opacity-30 disabled:hover:bg-transparent";

/// The data of a product detail page.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DetailData {
    product: Arc<Product>,
    category: Category,
    related: Vec<ProductSummary>,
}

/// Loads a product with its category and related products.
#[server]
pub async fn load_product(id: String) -> Result<DetailData, PageError> {
    let store = expect_context::<Arc<crate::store::Store>>();
    let product = id
        .parse()
        .ok()
        .and_then(|id| store.product(id))
        .ok_or(PageError::NotFound)?;
    Ok(DetailData {
        product: product.clone(),
        category: store.category_of(product).clone(),
        related: store
            .related(product)
            .map(|related| store.summary(related))
            .collect(),
    })
}

/// `GET /products/{id}`: product details, specifications, reviews and related products.
#[component]
pub fn DetailPage() -> impl IntoView {
    let params = use_params_map();
    let data = Resource::new(
        move || params.with(|params| params.get("id").unwrap_or_default()),
        load_product,
    );
    view! {
        <Suspense>
            {move || Suspend::new(async move {
                match data.await {
                    Ok(data) => Either::Left(view! { <Detail data /> }),
                    Err(error) => Either::Right(view! { <ErrorPage error /> }),
                }
            })}
        </Suspense>
    }
}

#[component]
fn Detail(data: DetailData) -> impl IntoView {
    let DetailData {
        product,
        category,
        related,
    } = data;
    let category_href = ListingUrl::category(&category.slug).to_string();
    let rating = Rating(product.rating).to_string();
    let filled = filled_stars(product.rating_tenths());
    let review_count = Count(product.review_count.into()).to_string();
    view! {
        <PageTitle title=product.name.clone() />
        <div class="mx-auto max-w-7xl px-4 py-10 sm:px-6 lg:px-8">
            <Breadcrumbs
                trail=vec![
                    ("Home".to_owned(), "/".to_owned()),
                    ("Products".to_owned(), "/products".to_owned()),
                    (category.name.clone(), category_href.clone()),
                ]
                current=product.name.clone()
            />
            <div class="mt-8 grid gap-10 lg:grid-cols-2 lg:gap-16">
                <div class="lg:sticky lg:top-24 lg:self-start">
                    <ProductImage
                        color=product.color
                        on_sale=product.compare_at_cents.is_some()
                        sold_out=product.is_sold_out()
                    />
                </div>
                <div>
                    <a
                        href=category_href
                        class="text-sm font-semibold text-teal-700 hover:text-teal-900"
                    >
                        {category.name}
                    </a>
                    <h1 class="mt-2 text-3xl font-semibold tracking-tight text-balance text-stone-900 sm:text-4xl">
                        {product.name.clone()}
                    </h1>
                    <div class="mt-3 flex items-center gap-2">
                        <Stars filled rating=rating.clone() size=StarSize::Large />
                        <span class="text-sm font-medium text-stone-900">{rating.clone()}</span>
                        <a
                            href="#reviews"
                            class="text-sm text-stone-500 underline-offset-4 hover:text-stone-900 hover:underline"
                        >
                            {format!("{review_count} reviews")}
                        </a>
                    </div>
                    <div class="mt-6 flex flex-wrap items-baseline gap-3">
                        <p class="text-3xl font-semibold tracking-tight text-stone-900">
                            {Money(product.price_cents.into()).to_string()}
                        </p>
                        {product
                            .compare_at_cents
                            .map(|compare_at| {
                                let saving = compare_at - product.price_cents;
                                view! {
                                    <s class="text-lg text-stone-400">
                                        {Money(compare_at.into()).to_string()}
                                    </s>
                                    <span class="rounded-full bg-rose-50 px-2.5 py-1 text-sm font-medium text-rose-700">
                                        {format!(
                                            "Save {} ({}%)",
                                            Money(saving.into()),
                                            saving * 100 / compare_at,
                                        )}
                                    </span>
                                }
                            })}
                    </div>
                    <p class="mt-4 text-lg leading-8 text-stone-600">{product.tagline.clone()}</p>
                    <dl class="mt-6 grid grid-cols-[auto_1fr] gap-x-8 gap-y-3 text-sm">
                        <dt class="font-medium text-stone-900">"Color"</dt>
                        <dd class="flex items-center gap-2 text-stone-600">
                            <span class=format!(
                                "size-4 rounded-full ring-1 ring-black/10 {}",
                                Palette::of(product.color).accent,
                            )></span>
                            {product.color.label()}
                        </dd>
                        <dt class="font-medium text-stone-900">"Availability"</dt>
                        <Availability stock=product.stock />
                    </dl>
                    {if product.is_sold_out() {
                        Either::Left(
                            view! {
                                <div class="mt-8">
                                    <button
                                        type="button"
                                        disabled
                                        class="w-full cursor-not-allowed rounded-full bg-stone-200 px-8 py-3.5 text-base font-semibold text-stone-500"
                                    >
                                        "Sold out"
                                    </button>
                                </div>
                            },
                        )
                    } else {
                        Either::Right(view! { <AddToCart max=product.max_quantity() /> })
                    }}
                    <ul class="mt-8 space-y-3 border-t border-stone-200 pt-6 text-sm text-stone-600">
                        <li class="flex items-center gap-3">
                            {icons::check()}
                            "Free shipping on orders over $75"
                        </li>
                        <li class="flex items-center gap-3">{icons::check()} "Free 30-day returns"</li>
                        <li class="flex items-center gap-3">
                            {icons::check()}
                            "Ships within one business day"
                        </li>
                    </ul>
                </div>
            </div>
            <div class="mt-16 grid gap-12 border-t border-stone-200 pt-16 lg:grid-cols-2">
                <section>
                    <h2 class="text-xl font-semibold tracking-tight">"Description"</h2>
                    <div class="mt-4 space-y-4 leading-7 text-stone-600">
                        {product
                            .paragraphs()
                            .map(|paragraph| view! { <p>{paragraph.to_owned()}</p> })
                            .collect_view()}
                    </div>
                </section>
                <section>
                    <h2 class="text-xl font-semibold tracking-tight">"Specifications"</h2>
                    <table class="mt-4 w-full text-left text-sm">
                        <tbody class="divide-y divide-stone-200 border-y border-stone-200">
                            {product
                                .specs
                                .iter()
                                .map(|spec| {
                                    view! {
                                        <tr>
                                            <th
                                                scope="row"
                                                class="w-2/5 py-3 pr-4 font-medium text-stone-900"
                                            >
                                                {spec.label.clone()}
                                            </th>
                                            <td class="py-3 text-stone-600">{spec.value.clone()}</td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                </section>
            </div>
            <section
                id="reviews"
                class="mt-16 grid gap-10 border-t border-stone-200 pt-16 lg:grid-cols-[18rem_1fr]"
            >
                <div>
                    <h2 class="text-xl font-semibold tracking-tight">"Customer reviews"</h2>
                    <div class="mt-4 flex items-center gap-3">
                        <p class="text-4xl font-semibold tracking-tight">{rating.clone()}</p>
                        <Stars filled rating size=StarSize::Large />
                    </div>
                    <p class="mt-2 text-sm text-stone-600">
                        {format!("Based on {review_count} reviews")}
                    </p>
                    <p class="mt-1 text-sm text-stone-500">
                        {format!("Showing the {} most recent.", product.reviews.len())}
                    </p>
                </div>
                <ul class="divide-y divide-stone-200">
                    {product
                        .reviews
                        .iter()
                        .map(|review| {
                            view! {
                                <li class="py-6 first:pt-0">
                                    <article>
                                        <div class="flex items-center justify-between gap-4">
                                            <Stars
                                                filled=review.rating
                                                rating=review.rating.to_string()
                                                size=StarSize::Small
                                            />
                                            <time
                                                datetime=IsoDate(review.date).to_string()
                                                class="text-sm text-stone-500"
                                            >
                                                {ShortDate(review.date).to_string()}
                                            </time>
                                        </div>
                                        <h3 class="mt-3 font-semibold text-stone-900">
                                            {review.title.clone()}
                                        </h3>
                                        <p class="mt-2 leading-7 text-stone-600">{review.body.clone()}</p>
                                        <p class="mt-3 text-sm font-medium text-stone-900">
                                            {review.author.clone()}
                                        </p>
                                    </article>
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
            </section>
            <section aria-labelledby="related-heading" class="mt-16 border-t border-stone-200 pt-16">
                <h2 id="related-heading" class="text-xl font-semibold tracking-tight">
                    "You might also like"
                </h2>
                <div class="mt-8 grid grid-cols-2 gap-x-6 gap-y-10 lg:grid-cols-4">
                    {related
                        .into_iter()
                        .map(|product| view! { <ProductCard product /> })
                        .collect_view()}
                </div>
            </section>
        </div>
    }
}

#[component]
fn Availability(stock: u32) -> impl IntoView {
    match stock {
        0 => view! { <dd class="font-medium text-stone-500">"Sold out"</dd> }.into_any(),
        1..10 => view! {
            <dd class="font-medium text-amber-700">{format!("Only {stock} left in stock")}</dd>
        }
        .into_any(),
        _ => view! { <dd class="font-medium text-emerald-700">"In stock"</dd> }.into_any(),
    }
}

/// The quantity stepper and add-to-cart button of a product in stock.
///
/// Adding updates the `cart_count` cookie and the [`CartCount`] signal shown in the header.
#[component]
fn AddToCart(max: u32) -> impl IntoView {
    let cart = expect_context::<RwSignal<CartCount>>();
    let quantity = RwSignal::new(1_u32);
    let status = RwSignal::new(String::new());
    let step = move |delta: i32| {
        quantity.update(|quantity| *quantity = quantity.saturating_add_signed(delta).clamp(1, max));
    };
    let add_to_cart = move |_| {
        let count = CartCount::from_document().add(quantity.get());
        count.save();
        cart.set(count);
        status.set(format!("Added {} to your cart.", quantity.get()));
    };
    view! {
        <div data-product-form data-max=max class="mt-8">
            <div class="flex flex-col gap-3 sm:flex-row">
                <div class="flex items-center justify-between rounded-full border border-stone-300 bg-white p-1 sm:w-36">
                    <button
                        type="button"
                        aria-label="Decrease quantity"
                        disabled={move || quantity.get() <= 1}
                        class=STEPPER_BUTTON
                        on:click=move |_| step(-1)
                    >
                        {icons::minus()}
                    </button>
                    <output
                        aria-live="polite"
                        aria-label="Quantity"
                        class="w-10 text-center font-semibold tabular-nums"
                    >
                        {quantity}
                    </output>
                    <button
                        type="button"
                        aria-label="Increase quantity"
                        disabled={move || quantity.get() >= max}
                        class=STEPPER_BUTTON
                        on:click=move |_| step(1)
                    >
                        {icons::plus()}
                    </button>
                </div>
                <button
                    type="button"
                    class="flex-1 rounded-full bg-teal-700 px-8 py-3.5 text-base font-semibold text-white shadow-sm hover:bg-teal-800 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-teal-700"
                    on:click=add_to_cart
                >
                    "Add to cart"
                </button>
            </div>
            <p role="status" class="mt-3 min-h-5 text-sm font-medium text-teal-800">
                {status}
            </p>
        </div>
    }
}
