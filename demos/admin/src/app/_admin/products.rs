mod product_id;

use serde::Serialize;
use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    l10n::{Translation, fluent::messages},
    router::{Slot, href, layout, page, query_params},
    runtime::{Event, Signal, link, shard, signal},
    view::{StaticClass, View, attributes, class, component, element_id, suspense, view},
};

use self::product_id::ProductId;
use super::Section;
use crate::{
    app::_common::money,
    components::{
        badge::{BadgeVariant, badge},
        button::{ButtonSize, ButtonVariant, button},
        input::input,
        list_pagination::{ListPage, list_pagination},
        select::select,
        skeleton::skeleton,
        table::{table, table_body, table_cell, table_head, table_header, table_row},
    },
    features::{
        auth::require_user,
        catalog::{ProductSearch, ProductSort, StockLevel},
    },
};

messages!("{mod}/l10n/{locale}.ftl");

const PER_PAGE: usize = 20;

/// URL parameters for the initial list. Pagination clicks update a signal without
/// changing the URL; opening a page link in a new tab uses these parameters.
#[query_params(error = redirect("?"))]
#[derive(Serialize)]
pub struct ProductsQuery {
    q: Option<String>,
    stock: Option<StockLevel>,
    sort: Option<ProductSort>,
    page: Option<usize>,
}

impl ProductsQuery {
    fn new(text: &str, stock: Option<StockLevel>, sort: ProductSort, number: usize) -> Self {
        Self {
            q: (!text.is_empty()).then(|| text.to_owned()),
            stock,
            sort: (sort != ProductSort::default()).then_some(sort),
            page: (number > 1).then_some(number),
        }
    }
}

/// Marks the product pages as part of the products section.
#[layout(with(Section::Products))]
async fn section(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! { (slot) })
}

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let query = query_params::<ProductsQuery>(cx)?;
    let initial_stock = query.stock.map_or("", StockLevel::slug);
    let initial_sort = query.sort.unwrap_or_default().slug();

    let search = signal(cx, || query.q.clone().unwrap_or_default());
    let stock = signal(cx, || initial_stock.to_owned());
    let sort = signal(cx, || initial_sort.to_owned());
    let page_number = signal(cx, || query.page.unwrap_or(1));
    let search_id = element_id(cx);
    let stock_id = element_id(cx);
    let sort_id = element_id(cx);

    Ok(view! {
        <section
            class="overflow-hidden rounded-xl border border-border bg-card \
                text-card-foreground shadow-xs"
        >
            <div class="flex flex-col gap-2 p-3 sm:flex-row sm:items-center">
                <div class="relative sm:max-w-xs sm:flex-1">
                    <label for=(&search_id) class="sr-only">(t!(search))</label>
                    icon(
                        data: iconify_icon!("lucide:search"),
                        attrs: attributes! {
                            class="pointer-events-none absolute top-1/2 left-3 size-4 \
                                -translate-y-1/2 text-muted-foreground"
                        }
                    )
                    input(
                        attrs: attributes! {
                            id=(&search_id)
                            type="search"
                            placeholder=(t!(search.placeholder))
                            class="pl-9"
                            :value=$(search.get())
                            @input=$(|e: Event| {
                                search.set(e.target.value);
                                page_number.set(1);
                            })
                        }
                    )
                </div>
                <div class="grid grid-cols-2 gap-2 sm:ml-auto sm:flex">
                    <label for=(&stock_id) class="sr-only">(t!(stock_filter))</label>
                    select(
                        attrs: attributes! {
                            id=(&stock_id)
                            class="sm:w-40"
                            :value=$(stock.get())
                            @change=$(|e: Event| {
                                stock.set(e.target.value);
                                page_number.set(1);
                            })
                        },
                        <option value="" selected=(initial_stock.is_empty())>
                            (t!(stock_filter.all))
                        </option>
                        for option in StockLevel::ALL {
                            <option
                                value=(option.slug())
                                selected=(initial_stock == option.slug())
                            >
                                (stock_label(option))
                            </option>
                        }
                    )
                    <label for=(&sort_id) class="sr-only">(t!(sort))</label>
                    select(
                        attrs: attributes! {
                            id=(&sort_id)
                            class="sm:w-40"
                            :value=$(sort.get())
                            @change=$(|e: Event| {
                                sort.set(e.target.value);
                                page_number.set(1);
                            })
                        },
                        for option in ProductSort::ALL {
                            <option
                                value=(option.slug())
                                selected=(initial_sort == option.slug())
                            >
                                (sort_label(option))
                            </option>
                        }
                    )
                </div>
            </div>

            product_list(
                search: search.clone(),
                stock: stock.clone(),
                sort: sort.clone(),
                page_number: page_number.clone()
            )
        </section>
    })
}

#[shard]
async fn product_list(
    cx: &Cx,
    search: Signal<String>,
    stock: Signal<String>,
    sort: Signal<String>,
    page_number: Signal<usize>,
) -> Result<impl View> {
    // Shards can be requested without the page layout, so they need their own sign-in check.
    require_user(cx).await?;

    let text = search.get();
    let selected = stock.get().parse::<StockLevel>().ok();
    let sort = sort.get().parse::<ProductSort>().unwrap_or_default();
    let count = ProductSearch {
        stock: selected,
        text: &text,
    }
    .count(cx)
    .await?;

    let shown = ListPage::new(count, PER_PAGE, page_number.get());

    Ok(view! {
        if count == 0 {
            <div class="flex flex-col items-center gap-3 px-6 py-16 text-center">
                <span
                    class="flex size-10 items-center justify-center rounded-full \
                        bg-foreground/5 text-muted-foreground"
                >
                    icon(data: iconify_icon!("lucide:package-search"))
                </span>
                <div class="flex flex-col gap-1">
                    <p class="font-medium">(t!(no_results))</p>
                    <p class="text-sm text-muted-foreground">
                        (t!(no_results.description))
                    </p>
                </div>
                button(
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    attrs: attributes! {
                        type="button"
                        @click=$(|_e: Event| {
                            search.set("".to_owned());
                            stock.set("".to_owned());
                            page_number.set(1);
                        })
                    },
                    (t!(clear_filters))
                )
            </div>
        } else {
            suspense(
                fallback: view! { product_rows_skeleton(rows: shown.items()) },
                product_rows(
                    filter: ProductSearch { stock: selected, text: &text },
                    sort: sort,
                    number: shown.number
                )
            )

            list_pagination(
                page: shown,
                href: |number| {
                    href!(page)
                        .query(&ProductsQuery::new(&text, selected, sort, number))
                        .resolve(cx)
                },
                number: &page_number
            )
        }
    })
}

#[component]
async fn product_rows(
    cx: &Cx,
    filter: ProductSearch<'_>,
    sort: ProductSort,
    number: usize,
) -> Result<impl View> {
    let products = filter.page(cx, sort, number, PER_PAGE).await?;

    Ok(view! {
        table(
            product_columns()
            table_body(
                for product in &products {
                    table_row(
                        attrs: attributes! { class="relative" },
                        table_cell(
                            <div class="flex max-w-80 min-w-0 flex-col">
                                link(
                                    href: href!(product_id::page, ProductId(product.id)),
                                    attrs: attributes! {
                                        class="truncate font-medium after:absolute after:inset-0"
                                    },
                                    (&product.name)
                                )
                                <span class="font-mono text-xs text-muted-foreground">
                                    (&product.sku)
                                </span>
                            </div>
                        )
                        table_cell(stock_badge(level: product.stock_level()))
                        table_cell(
                            attrs: attributes! { class="text-right text-muted-foreground tabular-nums" },
                            (product.stock)
                        )
                        table_cell(
                            attrs: attributes! { class="text-right font-medium tabular-nums" },
                            (money(product.price))
                        )
                    )
                }
            )
        )
    })
}

#[component]
async fn product_rows_skeleton(rows: usize) -> Result<impl View> {
    Ok(view! {
        table(
            attrs: attributes! { aria-busy="true" },
            product_columns()
            table_body(
                for _ in 0..rows {
                    table_row(
                        table_cell(
                            <div class="flex max-w-80 min-w-0 flex-col">
                                <div class="flex h-5 items-center">
                                    skeleton(
                                        attrs: attributes! { class="h-3.5 w-40 max-w-full" }
                                    )
                                </div>
                                <div class="flex h-4 items-center">
                                    skeleton(attrs: attributes! { class="h-3 w-16" })
                                </div>
                            </div>
                        )
                        table_cell(skeleton(attrs: attributes! { class="h-5.5 w-20" }))
                        table_cell(
                            skeleton(attrs: attributes! { class="ml-auto h-4 w-8" })
                        )
                        table_cell(
                            skeleton(attrs: attributes! { class="ml-auto h-4 w-16" })
                        )
                    )
                }
            )
        )
    })
}

#[component]
async fn product_columns() -> Result<impl View> {
    Ok(view! {
        table_header(
            table_row(
                table_head(attrs: attributes! { class="w-full" }, (t!(column_product)))
                table_head((t!(column_status)))
                table_head(
                    attrs: attributes! { class="text-right" },
                    (t!(column_stock))
                )
                table_head(
                    attrs: attributes! { class="text-right" },
                    (t!(column_price))
                )
            )
        )
    })
}

#[component]
async fn stock_badge(level: StockLevel) -> Result<impl View> {
    let dot: StaticClass = match level {
        StockLevel::InStock => class!("bg-emerald-500"),
        StockLevel::Low => class!("bg-amber-500"),
        StockLevel::Out => class!("bg-rose-500"),
    };

    Ok(view! {
        badge(
            variant: BadgeVariant::Outline,
            <span class=(class!("size-1.5 rounded-full", dot))></span>
            (stock_label(level))
        )
    })
}

fn stock_label(level: StockLevel) -> Translation<'static> {
    match level {
        StockLevel::InStock => t!(stock_in_stock),
        StockLevel::Low => t!(stock_low),
        StockLevel::Out => t!(stock_out),
    }
}

fn sort_label(sort: ProductSort) -> Translation<'static> {
    match sort {
        ProductSort::Name => t!(sort_name),
        ProductSort::HighestPrice => t!(sort_highest_price),
        ProductSort::LowestPrice => t!(sort_lowest_price),
        ProductSort::LowestStock => t!(sort_lowest_stock),
    }
}
