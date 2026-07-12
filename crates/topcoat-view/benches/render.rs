//! Micro-benchmarks for [`View::render`].
//!
//! Each scenario builds a [`View`] once, up front, and then times only the
//! render pass that serializes it into an HTML `String`. Constructing the view
//! (running components, evaluating `format!` interpolations, and pushing view
//! parts) happens in setup, so the measured work is the final tree walk:
//! writing static markup, escaping dynamic text, and formatting numbers into
//! the output buffer.
//!
//! The scenarios isolate distinct cost centers -- static markup, text escaping,
//! numeric formatting, and attribute output -- and finish with a realistic,
//! component-based product grid parameterized by size to show how rendering
//! scales with document length.

use std::future::Future;
use std::hint::black_box;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion, Throughput, criterion_group, criterion_main};

use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

/// Drives a future to completion on the current thread.
///
/// The futures produced by the `view!` macro never yield: every `.await` inside
/// them resolves immediately, so a single poll completes them. The loop is only
/// a safety net and does not busy-wait in practice.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
}

/// Times `view.render(cx)` and reports throughput as the rendered byte length,
/// so the report shows bytes per second alongside per-render latency.
fn measure(group: &mut BenchmarkGroup<'_, WallTime>, id: impl Into<String>, cx: &Cx, view: &View) {
    group.throughput(Throughput::Bytes(view.render(cx).len() as u64));
    group.bench_function(id.into(), |b| {
        b.iter(|| black_box(black_box(view).render(black_box(cx))));
    });
}

// ---------------------------------------------------------------------------
// Static markup
// ---------------------------------------------------------------------------

/// A page made entirely of static markup: no interpolation, no escaping, no
/// control flow. Isolates the cost of emitting literal HTML and growing the
/// output buffer, which is the fast path the renderer takes most often.
async fn static_page() -> Result {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>"Meridian Supply"</title>
            </head>
            <body class="bg-slate-50 text-slate-900">
                <header class="border-b border-slate-200 bg-white">
                    <nav
                        class="mx-auto flex w-full max-w-6xl items-center justify-between px-4 py-4"
                    >
                        <a href="/" class="text-lg font-bold tracking-tight">
                            "Meridian Supply"
                        </a>
                        <div
                            class="flex items-center gap-6 text-sm font-medium text-slate-600"
                        >
                            <a href="/" class="hover:text-slate-900">"Home"</a>
                            <a href="/products" class="hover:text-slate-900">
                                "Products"
                            </a>
                            <a href="/about" class="hover:text-slate-900">"About"</a>
                        </div>
                    </nav>
                </header>
                <main class="mx-auto w-full max-w-6xl px-4 py-16">
                    <section class="grid gap-8 md:grid-cols-2">
                        <div>
                            <p
                                class="text-sm font-semibold uppercase tracking-wide text-indigo-600"
                            >
                                "New collection"
                            </p>
                            <h1 class="mt-3 text-4xl font-bold tracking-tight">
                                "Gear that keeps up with your workflow"
                            </h1>
                            <p class="mt-4 text-lg text-slate-600">
                                "Precision-built peripherals, displays, and audio for people who
                                spend their day at a desk and expect their tools to disappear into
                                the work."
                            </p>
                            <div class="mt-6 flex gap-3">
                                <a
                                    href="/products"
                                    class="rounded-md bg-indigo-600 px-4 py-2 font-medium text-white"
                                >
                                    "Shop everything"
                                </a>
                                <a
                                    href="/products?category=audio"
                                    class="rounded-md border border-slate-300 px-4 py-2 font-medium"
                                >
                                    "Browse audio"
                                </a>
                            </div>
                        </div>
                        <div
                            class="flex h-64 items-center justify-center rounded-2xl bg-slate-100 text-6xl font-bold text-slate-300"
                        >
                            "MS"
                        </div>
                    </section>
                    <section class="mt-20 grid gap-6 md:grid-cols-3">
                        <article
                            class="rounded-xl border border-slate-200 bg-white p-6"
                        >
                            <h3 class="text-lg font-semibold">
                                "Free two-day shipping"
                            </h3>
                            <p class="mt-2 text-sm text-slate-600">
                                "Every order over fifty dollars ships free, tracked, and carbon
                                neutral from our regional warehouses."
                            </p>
                        </article>
                        <article
                            class="rounded-xl border border-slate-200 bg-white p-6"
                        >
                            <h3 class="text-lg font-semibold">"Two-year warranty"</h3>
                            <p class="mt-2 text-sm text-slate-600">
                                "We stand behind the build quality of everything we sell with a
                                no-questions replacement policy."
                            </p>
                        </article>
                        <article
                            class="rounded-xl border border-slate-200 bg-white p-6"
                        >
                            <h3 class="text-lg font-semibold">"Thirty-day returns"</h3>
                            <p class="mt-2 text-sm text-slate-600">
                                "Not the right fit? Send it back within thirty days for a full
                                refund, no restocking fees."
                            </p>
                        </article>
                    </section>
                </main>
                <footer class="border-t border-slate-200 bg-white">
                    <p
                        class="mx-auto w-full max-w-6xl px-4 py-6 text-xs text-slate-400"
                    >
                        "(c) 2026 Meridian Supply. All rights reserved."
                    </p>
                </footer>
            </body>
        </html>
    }
}

// ---------------------------------------------------------------------------
// Text escaping
// ---------------------------------------------------------------------------

/// A feed of user comments rendered as escaped text. Each comment is peppered
/// with HTML-significant characters (`<`, `>`, `&`, quotes), so this scenario
/// exercises the escaping path in the formatter rather than the bulk copy of
/// safe runs.
async fn comment_feed(cx: &Cx, comments: &[String]) -> Result {
    view! {
        cx =>
        <ul class="space-y-4">
            for comment in comments {
                <li
                    class="rounded-lg border border-slate-200 p-4 text-sm text-slate-700"
                >
                    (comment)
                </li>
            }
        </ul>
    }
}

fn make_comments(count: usize) -> Vec<String> {
    // A mix of prose and markup-like fragments so most of each string is a safe
    // run punctuated by characters that must be escaped.
    (0..count)
        .map(|i| {
            format!(
                "Reviewer #{i} said: \"5 < 10 && this <widget> is great\" -- \
                 compare <b>value</b> & price at <https://example.com/p?id={i}&ref=feed>."
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Numeric formatting
// ---------------------------------------------------------------------------

/// A dense table of integers. Every cell formats a number through the
/// `Display`-based render path, isolating numeric formatting from markup.
async fn numeric_table(cx: &Cx, rows: &[Vec<i64>]) -> Result {
    view! {
        cx =>
        <table class="w-full text-right font-mono text-sm">
            <tbody>
                for row in rows {
                    <tr>
                        for value in row {
                            <td class="px-2 py-1 tabular-nums">(value)</td>
                        }
                    </tr>
                }
            </tbody>
        </table>
    }
}

fn make_number_rows(rows: usize, cols: usize) -> Vec<Vec<i64>> {
    (0..rows)
        .map(|r| {
            (0..cols)
                .map(|c| {
                    // Spread values across magnitudes and signs so the numeric
                    // formatter handles a realistic range of widths.
                    let n = i64::try_from(r * cols + c).unwrap_or_default();
                    (n * 2_654_435_761) % 1_000_000 - 500_000
                })
                .collect()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Attribute-heavy output
// ---------------------------------------------------------------------------

/// One element per item, each carrying several dynamic attributes. Isolates the
/// attribute-writing path: name emission, value escaping, and the surrounding
/// quoting.
async fn tag_cloud(cx: &Cx, tags: &[Tag]) -> Result {
    view! {
        cx =>
        <div class="flex flex-wrap gap-2">
            for tag in tags {
                <a
                    href=(&tag.href)
                    id=(&tag.id)
                    class=(&tag.class)
                    title=(&tag.title)
                    data-count=(tag.count)
                    data-slug=(&tag.slug)
                    aria-label=(&tag.title)
                >
                    (&tag.label)
                </a>
            }
        </div>
    }
}

struct Tag {
    id: String,
    href: String,
    slug: String,
    label: String,
    title: String,
    class: String,
    count: u32,
}

fn make_tags(count: usize) -> Vec<Tag> {
    const TONES: [&str; 4] = ["indigo", "emerald", "amber", "rose"];
    (0..count)
        .map(|i| {
            let tone = TONES[i % TONES.len()];
            Tag {
                id: format!("tag-{i}"),
                href: format!("/tags/{i}?sort=recent"),
                slug: format!("tag-{i}-slug"),
                label: format!("Topic {i}"),
                title: format!("All posts tagged \"Topic {i}\""),
                class: format!(
                    "inline-flex rounded-full bg-{tone}-100 px-3 py-1 text-sm font-medium text-{tone}-700"
                ),
                count: (u32::try_from(i).unwrap_or_default() * 7) % 500,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Realistic component-based page
// ---------------------------------------------------------------------------

/// The heroicons solid star, as used by the storefront benchmark.
const STAR_PATH: &str = "M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.539 1.118l-2.8-2.034a1 1 0 00-1.176 0l-2.8 2.034c-.783.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.363-1.118l-2.8-2.034c-.784-.57-.381-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z";

struct Product {
    id: u32,
    name: String,
    category: String,
    price_cents: u32,
    rating_tenths: u32,
    review_count: u32,
}

fn format_price(cents: u32) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

fn format_rating(tenths: u32) -> String {
    format!("{}.{}", tenths / 10, tenths % 10)
}

fn make_products(count: usize) -> Vec<Product> {
    const CATEGORIES: [&str; 3] = ["Audio", "Displays", "Wearables"];
    (0..count)
        .map(|i| {
            let n = u32::try_from(i).unwrap_or_default();
            Product {
                id: 1000 + n,
                // Quotes and an ampersand keep the realistic path exercising the
                // escaper the way real product names do.
                name: format!("Meridian \"Model {i}\" & Co."),
                category: CATEGORIES[i % CATEGORIES.len()].to_string(),
                price_cents: 1999 + (n % 40) * 500,
                rating_tenths: 30 + (n % 20),
                review_count: 3 + (n * 7) % 400,
            }
        })
        .collect()
}

/// A row of five stars, filled up to the product's rating. Nested inside every
/// card, so it multiplies the per-card render work.
#[component]
async fn rating_stars(tenths: u32) -> Result {
    let filled = (tenths + 5) / 10;

    view! {
        <div class="flex">
            for index in 0..5u32 {
                <svg
                    viewBox="0 0 20 20"
                    fill="currentColor"
                    aria-hidden="true"
                    class=(if index < filled {
                        "h-4 w-4 text-amber-400"
                    } else {
                        "h-4 w-4 text-slate-200"
                    })
                >
                    <path d=(STAR_PATH)></path>
                </svg>
            }
        </div>
    }
}

/// A single product card: static markup mixed with escaped text, a nested
/// component, numeric interpolation, and computed attributes.
#[component]
async fn product_card(product: &Product) -> Result {
    view! {
        <a
            href=(format!("/products/{}", product.id))
            class="group flex flex-col rounded-xl border border-slate-200 bg-white p-4 shadow-sm transition hover:shadow-md"
        >
            <div
                class="mb-4 flex h-32 items-center justify-center rounded-lg bg-slate-100 text-3xl font-bold text-slate-300"
            >
                (&product.category)
            </div>
            <p class="text-xs font-medium uppercase tracking-wide text-slate-400">
                (&product.category)
            </p>
            <h3
                class="mt-1 text-sm font-semibold text-slate-900 group-hover:text-indigo-600"
            >
                (&product.name)
            </h3>
            <div class="mt-2 flex items-center gap-1">
                rating_stars(tenths: product.rating_tenths)
                <span class="text-xs text-slate-500">
                    (format_rating(product.rating_tenths))
                    " ("
                    (product.review_count)
                    ")"
                </span>
            </div>
            <p class="mt-3 text-lg font-bold">(format_price(product.price_cents))</p>
        </a>
    }
}

/// A responsive grid of product cards, the flagship realistic scenario.
///
/// Like the leaf scenarios above, it threads the request context into `view!`
/// with the `cx,` form; on top of that it invokes a component (`product_card`).
async fn product_grid(cx: &Cx, products: &[Product]) -> Result {
    view! {
        cx =>
        <main class="mx-auto w-full max-w-6xl px-4 py-10">
            <h1 class="text-2xl font-bold tracking-tight">"All products"</h1>
            <div class="mt-6 grid grid-cols-2 gap-4 md:grid-cols-3 lg:grid-cols-4">
                for product in products {
                    product_card(product: product)
                }
            </div>
        </main>
    }
}

// ---------------------------------------------------------------------------
// Benchmark entry point
// ---------------------------------------------------------------------------

fn bench_render(c: &mut Criterion) {
    let cx = Cx::default();
    let mut group = c.benchmark_group("view_render");

    let static_view = block_on(static_page()).expect("render static_page");
    measure(&mut group, "static_page", &cx, &static_view);

    let comments = make_comments(200);
    let comment_view = block_on(comment_feed(&cx, &comments)).expect("render comment_feed");
    measure(&mut group, "text_escaping", &cx, &comment_view);

    let number_rows = make_number_rows(120, 10);
    let number_view = block_on(numeric_table(&cx, &number_rows)).expect("render numeric_table");
    measure(&mut group, "numeric_table", &cx, &number_view);

    let tags = make_tags(200);
    let tag_view = block_on(tag_cloud(&cx, &tags)).expect("render tag_cloud");
    measure(&mut group, "attributes", &cx, &tag_view);

    // The realistic grid grows with the number of cards, showing how render
    // time scales with document length.
    for &count in &[12usize, 96, 768] {
        let products = make_products(count);
        let grid_view = block_on(product_grid(&cx, &products)).expect("render product_grid");
        measure(&mut group, format!("product_grid/{count}"), &cx, &grid_view);
    }

    group.finish();
}

criterion_group!(benches, bench_render);
criterion_main!(benches);
