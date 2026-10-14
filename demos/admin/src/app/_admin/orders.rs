pub(super) mod order_id;

use jiff::Timestamp;
use serde::Serialize;
use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::{href, page, query_params},
    runtime::{Event, Signal, link, shard, signal},
    view::{StaticClass, View, attributes, class, component, element_id, suspense, view},
};

use self::order_id::OrderId;
use crate::{
    components::{
        avatar::{AvatarSize, avatar, avatar_fallback},
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
        order::{OrderSearch, OrderSort, OrderStatus},
    },
};

const PER_PAGE: usize = 20;

/// URL parameters for the initial list. Pagination clicks update a signal without
/// changing the URL; opening a page link in a new tab uses these parameters.
#[query_params(error = redirect("?"))]
#[derive(Serialize)]
pub struct OrdersQuery {
    q: Option<String>,
    status: Option<OrderStatus>,
    sort: Option<OrderSort>,
    page: Option<usize>,
}

impl OrdersQuery {
    fn new(text: &str, status: Option<OrderStatus>, sort: OrderSort, number: usize) -> Self {
        Self {
            q: (!text.is_empty()).then(|| text.to_owned()),
            status,
            sort: (sort != OrderSort::default()).then_some(sort),
            page: (number > 1).then_some(number),
        }
    }
}

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let query = query_params::<OrdersQuery>(cx)?;
    let initial_status = query.status.map_or("", OrderStatus::slug);
    let initial_sort = query.sort.unwrap_or_default().slug();

    let search = signal(cx, || query.q.clone().unwrap_or_default());
    let status = signal(cx, || initial_status.to_owned());
    let sort = signal(cx, || initial_sort.to_owned());
    let page_number = signal(cx, || query.page.unwrap_or(1));
    let search_id = element_id(cx);
    let status_id = element_id(cx);
    let sort_id = element_id(cx);

    Ok(view! {
        <section
            class="overflow-hidden rounded-xl border border-border bg-card \
                text-card-foreground shadow-xs"
        >
            <div class="flex flex-col gap-2 p-3 sm:flex-row sm:items-center">
                <div class="relative sm:max-w-xs sm:flex-1">
                    <label for=(&search_id) class="sr-only">"Search orders"</label>
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
                            placeholder="Search customers or order numbers"
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
                    <label for=(&status_id) class="sr-only">"Status"</label>
                    select(
                        attrs: attributes! {
                            id=(&status_id)
                            class="sm:w-40"
                            :value=$(status.get())
                            @change=$(|e: Event| {
                                status.set(e.target.value);
                                page_number.set(1);
                            })
                        },
                        <option value="" selected=(initial_status.is_empty())>
                            "All statuses"
                        </option>
                        for option in OrderStatus::ALL {
                            <option
                                value=(option.slug())
                                selected=(initial_status == option.slug())
                            >
                                (option.label())
                            </option>
                        }
                    )
                    <label for=(&sort_id) class="sr-only">"Sort by"</label>
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
                        for option in OrderSort::ALL {
                            <option
                                value=(option.slug())
                                selected=(initial_sort == option.slug())
                            >
                                (option.label())
                            </option>
                        }
                    )
                </div>
            </div>

            order_list(
                search: search.clone(),
                status: status.clone(),
                sort: sort.clone(),
                page_number: page_number.clone()
            )
        </section>
    })
}

#[shard]
async fn order_list(
    cx: &Cx,
    search: Signal<String>,
    status: Signal<String>,
    sort: Signal<String>,
    page_number: Signal<usize>,
) -> Result<impl View> {
    // Shards can be requested without the page layout, so they need their own sign-in check.
    require_user(cx).await?;

    let text = search.get();
    let selected = status.get().parse::<OrderStatus>().ok();
    let sort = sort.get().parse::<OrderSort>().unwrap_or_default();
    let count = OrderSearch {
        status: selected,
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
                    <p class="font-medium">"No orders found"</p>
                    <p class="text-sm text-muted-foreground">
                        "No orders match your search and status."
                    </p>
                </div>
                button(
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    attrs: attributes! {
                        type="button"
                        @click=$(|_e: Event| {
                            search.set("".to_owned());
                            status.set("".to_owned());
                            page_number.set(1);
                        })
                    },
                    "Clear filters"
                )
            </div>
        } else {
            suspense(
                fallback: view! { order_rows_skeleton(rows: shown.items()) },
                order_rows(
                    filter: OrderSearch { status: selected, text: &text },
                    sort: sort,
                    number: shown.number
                )
            )

            list_pagination(
                page: shown,
                noun: "orders",
                href: |number| {
                    href!(page)
                        .query(&OrdersQuery::new(&text, selected, sort, number))
                        .resolve(cx)
                },
                number: &page_number
            )
        }
    })
}

#[component]
async fn order_rows(
    cx: &Cx,
    filter: OrderSearch<'_>,
    sort: OrderSort,
    number: usize,
) -> Result<impl View> {
    let orders = filter.page(cx, sort, number, PER_PAGE).await?;

    Ok(view! {
        table(
            order_columns()
            table_body(
                for order in &orders {
                    let customer = order.customer.get();
                    let units = order.units();
                    table_row(
                        attrs: attributes! { class="relative" },
                        table_cell(
                            attrs: attributes! { class="font-medium tabular-nums" },
                            link(
                                href: href!(order_id::page, OrderId(order.id)),
                                attrs: attributes! { class="after:absolute after:inset-0" },
                                "#"
                                (order.id)
                            )
                        )
                        table_cell(
                            <div class="flex items-center gap-3">
                                avatar(
                                    size: AvatarSize::Sm,
                                    avatar_fallback((customer.initials()))
                                )
                                <div class="flex max-w-64 min-w-0 flex-col">
                                    <span class="truncate font-medium">(&customer.name)</span>
                                    <span class="truncate text-xs text-muted-foreground">
                                        (&customer.email)
                                    </span>
                                </div>
                            </div>
                        )
                        table_cell(status_badge(status: order.status))
                        table_cell(
                            attrs: attributes! { class="text-muted-foreground tabular-nums" },
                            (units)
                            if units == 1 {
                                " item"
                            } else {
                                " items"
                            }
                        )
                        table_cell(
                            attrs: attributes! { class="text-muted-foreground" },
                            <time
                                datetime=(order.placed_at.to_string())
                                title=(order
                                    .placed_at
                                    .strftime("%b %-d, %Y at %H:%M UTC")
                                    .to_string())
                            >
                                (time_ago(order.placed_at))
                            </time>
                        )
                        table_cell(
                            attrs: attributes! { class="text-right font-medium tabular-nums" },
                            (order.total)
                        )
                    )
                }
            )
        )
    })
}

#[component]
async fn order_rows_skeleton(rows: usize) -> Result<impl View> {
    Ok(view! {
        table(
            attrs: attributes! { aria-busy="true" },
            order_columns()
            table_body(
                for _ in 0..rows {
                    table_row(
                        table_cell(skeleton(attrs: attributes! { class="h-4 w-10" }))
                        table_cell(
                            <div class="flex items-center gap-3">
                                avatar(
                                    size: AvatarSize::Sm,
                                    skeleton(attrs: attributes! { class="size-full" })
                                )
                                <div class="flex max-w-64 min-w-0 flex-col">
                                    <div class="flex h-5 items-center">
                                        skeleton(
                                            attrs: attributes! { class="h-3.5 w-28 max-w-full" }
                                        )
                                    </div>
                                    <div class="flex h-4 items-center">
                                        skeleton(
                                            attrs: attributes! { class="h-3 w-44 max-w-full" }
                                        )
                                    </div>
                                </div>
                            </div>
                        )
                        table_cell(skeleton(attrs: attributes! { class="h-5.5 w-20" }))
                        table_cell(skeleton(attrs: attributes! { class="h-4 w-14" }))
                        table_cell(skeleton(attrs: attributes! { class="h-4 w-20" }))
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
async fn order_columns() -> Result<impl View> {
    Ok(view! {
        table_header(
            table_row(
                table_head("Order")
                table_head(attrs: attributes! { class="w-full" }, "Customer")
                table_head("Status")
                table_head("Items")
                table_head("Placed")
                table_head(attrs: attributes! { class="text-right" }, "Total")
            )
        )
    })
}

#[component]
pub(super) async fn status_badge(status: OrderStatus) -> Result<impl View> {
    let dot: StaticClass = match status {
        OrderStatus::Pending => class!("bg-amber-500"),
        OrderStatus::Paid => class!("bg-sky-500"),
        OrderStatus::Shipped => class!("bg-violet-500"),
        OrderStatus::Delivered => class!("bg-emerald-500"),
        OrderStatus::Cancelled => class!("bg-muted-foreground/60"),
        OrderStatus::Refunded => class!("bg-rose-500"),
    };

    Ok(view! {
        badge(
            variant: BadgeVariant::Outline,
            <span class=(class!("size-1.5 rounded-full", dot))></span>
            (status.label())
        )
    })
}

/// Formats recent timestamps as elapsed time, such as `3 hours ago`.
/// After a week, uses a date such as `Oct 12, 2026`.
pub(super) fn time_ago(timestamp: Timestamp) -> String {
    let minutes = Timestamp::now().duration_since(timestamp).as_mins();
    let (amount, unit) = if minutes < 60 {
        (minutes, "minute")
    } else if minutes < 24 * 60 {
        (minutes / 60, "hour")
    } else if minutes < 7 * 24 * 60 {
        (minutes / (24 * 60), "day")
    } else {
        return timestamp.strftime("%b %-d, %Y").to_string();
    };

    match amount {
        ..1 => "just now".to_owned(),
        1 => format!("1 {unit} ago"),
        _ => format!("{amount} {unit}s ago"),
    }
}
