//! The `_dashboard` group puts this page at `/`, under the admin layout.

use std::time::Duration;

use tokio::time::{MissedTickBehavior, interval};
use topcoat::{
    Result,
    context::Cx,
    l10n::{Translation, fluent::messages},
    router::{href, page},
    runtime::{connected, link, shard},
    view::{
        EmitToken, StaticClass, View, attributes, class, component, emit, live, suspense, view,
    },
};

use super::{
    Section,
    orders::{
        order_id::{self, OrderId},
        status_badge, time_ago,
    },
};
use crate::{
    app::_common::{money, timestamp},
    components::{
        avatar::{AvatarSize, avatar, avatar_fallback},
        card::{card, card_description, card_header, card_title},
        chart::{bar_chart, line_chart},
        skeleton::skeleton,
        table::{table, table_body, table_cell, table_head, table_header, table_row},
    },
    features::{
        auth::require_user,
        catalog::Money,
        order::Order,
        stats::{SalesStats, VisitorStats},
    },
};

messages!("{mod}/l10n/{locale}.ftl");

const TICK: Duration = Duration::from_secs(1);

#[page(with(Section::Dashboard))]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    let first_name = user.name.split_whitespace().next().unwrap_or(&user.name);

    Ok(view! {
        <header class="flex flex-col gap-1">
            <h1 class="text-2xl font-semibold tracking-tight">
                (t!(welcome, name: first_name))
            </h1>
            <p class="text-muted-foreground">(t!(welcome.description))</p>
        </header>

        todays_stats()

        <section class="flex flex-col gap-3">
            <h2 class="font-semibold">(t!(needs_attention))</h2>
            card(
                card_header(
                    card_title((t!(open_orders)))
                    card_description((t!(open_orders.description)))
                )
                suspense(fallback: view! { open_orders_skeleton() }, open_orders())
            )
        </section>
    })
}

#[shard]
async fn todays_stats(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;

    Ok(view! {
        <section class="flex flex-col gap-3">
            <div class="flex items-center gap-2">
                <h2 class="font-semibold">(t!(today))</h2>
                <span class="relative flex size-2" aria-hidden="true">
                    <span
                        class="absolute inline-flex size-full animate-ping rounded-full bg-emerald-500 opacity-75"
                    ></span>
                    <span
                        class="relative inline-flex size-2 rounded-full bg-emerald-500"
                    ></span>
                </span>
                <span class="text-sm text-muted-foreground">(t!(live))</span>
            </div>
            <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
                stat_card(
                    label: t!(revenue),
                    accent: class!("text-emerald-500"),
                    figure: revenue
                )
                stat_card(
                    label: t!(orders),
                    accent: class!("text-violet-500"),
                    figure: orders
                )
                stat_card(
                    label: t!(average_order),
                    accent: class!("text-amber-500"),
                    figure: average_order
                )
                stat_card(
                    label: t!(visitors),
                    accent: class!("text-sky-500"),
                    figure: visitors
                )
            </div>
        </section>
    })
}

#[derive(PartialEq, Eq)]
struct Figure {
    value: Value,
    /// Change from the same time yesterday, in tenths of a percent.
    change: Option<i64>,
    chart: Chart,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Value {
    Money(Money),
    Count(i64),
}

/// Chart values run from oldest to newest.
#[derive(PartialEq, Eq)]
enum Chart {
    Bars(Vec<i64>),
    Line(Vec<i64>),
}

fn revenue() -> Figure {
    let sales = SalesStats::now();
    Figure {
        value: Value::Money(sales.today.revenue),
        change: change(sales.today.revenue.0, sales.yesterday.revenue.0),
        chart: Chart::Bars(
            sales
                .by_minute
                .iter()
                .map(|sales| sales.revenue.0)
                .collect(),
        ),
    }
}

fn orders() -> Figure {
    let sales = SalesStats::now();
    Figure {
        value: Value::Count(sales.today.orders),
        change: change(sales.today.orders, sales.yesterday.orders),
        chart: Chart::Bars(sales.by_minute.iter().map(|sales| sales.orders).collect()),
    }
}

fn average_order() -> Figure {
    let sales = SalesStats::now();
    let today = sales.today.average_order();
    Figure {
        value: Value::Money(today),
        change: change(today.0, sales.yesterday.average_order().0),
        chart: Chart::Line(
            sales
                .by_minute
                .iter()
                .filter(|sales| sales.orders > 0)
                .map(|sales| sales.average_order().0)
                .collect(),
        ),
    }
}

fn visitors() -> Figure {
    let visitors = VisitorStats::now();
    Figure {
        value: Value::Count(visitors.current),
        change: change(visitors.current, visitors.yesterday),
        chart: Chart::Line(visitors.by_second),
    }
}

/// Polls `figure` every [`TICK`] and sends an update when the value changes.
/// The chart uses `accent` as its color.
#[component]
async fn stat_card(
    cx: &Cx,
    label: Translation<'_>,
    accent: StaticClass,
    figure: fn() -> Figure,
) -> Result<impl View> {
    Ok(view! {
        card(
            attrs: attributes! { class="gap-0 overflow-hidden pb-0" },
            <p class="px-6 text-sm text-muted-foreground">(label)</p>
            (live! {
                let mut ticks = interval(TICK);
                ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
                let mut shown = None;
                loop {
                    ticks.tick().await;
                    let current = figure();
                    if shown.as_ref() != Some(&current) {
                        emit! { figure_details(figure: &current, accent: accent.clone()) }?;
                        shown = Some(current);
                    }
                    if !connected(cx) {
                        break Ok(EmitToken);
                    }
                }
            })
        )
    })
}

#[component]
async fn figure_details(figure: &Figure, accent: StaticClass) -> Result<impl View> {
    Ok(view! {
        <p class="mt-1 px-6 text-3xl font-semibold tracking-tight tabular-nums">
            match figure.value {
                Value::Money(amount) => (money(amount)),
                Value::Count(count) => (t!(count, count)),
            }
        </p>
        if let Some(change) = figure.change {
            <p
                class="mt-2 flex items-center gap-1.5 px-6 text-xs text-muted-foreground"
            >
                if change >= 0 {
                    <span class="font-medium text-emerald-600">
                        (t!(increase, percent: percent(change)))
                    </span>
                } else {
                    <span class="font-medium text-rose-600">
                        (t!(decrease, percent: percent(change)))
                    </span>
                }
                (t!(compared_to_yesterday))
            </p>
        }
        <div class=(class!("mt-4 h-16", accent))>
            match &figure.chart {
                Chart::Bars(values) => bar_chart(values: values),
                Chart::Line(values) => line_chart(values: values),
            }
        </div>
    })
}

const OPEN_ORDERS: usize = 6;

#[component]
async fn open_orders(cx: &Cx) -> Result<impl View> {
    let orders = Order::awaiting_action(cx, OPEN_ORDERS).await?;

    Ok(view! {
        if orders.is_empty() {
            <p class="px-6 text-sm text-muted-foreground">(t!(no_open_orders))</p>
        } else {
            table(
                open_order_columns()
                table_body(
                    for order in &orders {
                        let customer = order.customer.get();
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
                                    <span class="max-w-64 truncate font-medium">
                                        (&customer.name)
                                    </span>
                                </div>
                            )
                            table_cell(status_badge(status: order.status))
                            table_cell(
                                attrs: attributes! { class="text-muted-foreground" },
                                <time
                                    datetime=(order.placed_at.to_string())
                                    title=(timestamp(order.placed_at))
                                >
                                    time_ago(timestamp: order.placed_at)
                                </time>
                            )
                            table_cell(
                                attrs: attributes! { class="text-right font-medium tabular-nums" },
                                (money(order.total))
                            )
                        )
                    }
                )
            )
        }
    })
}

#[component]
async fn open_orders_skeleton() -> Result<impl View> {
    Ok(view! {
        table(
            attrs: attributes! { aria-busy="true" },
            open_order_columns()
            table_body(
                for _ in 0..OPEN_ORDERS {
                    table_row(
                        table_cell(skeleton(attrs: attributes! { class="h-4 w-10" }))
                        table_cell(
                            <div class="flex items-center gap-3">
                                avatar(
                                    size: AvatarSize::Sm,
                                    skeleton(attrs: attributes! { class="size-full" })
                                )
                                skeleton(
                                    attrs: attributes! { class="h-3.5 w-28 max-w-full" }
                                )
                            </div>
                        )
                        table_cell(skeleton(attrs: attributes! { class="h-5.5 w-20" }))
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
async fn open_order_columns() -> Result<impl View> {
    Ok(view! {
        table_header(
            table_row(
                table_head((t!(column_order)))
                table_head(attrs: attributes! { class="w-full" }, (t!(column_customer)))
                table_head((t!(column_status)))
                table_head((t!(column_placed)))
                table_head(
                    attrs: attributes! { class="text-right" },
                    (t!(column_total))
                )
            )
        )
    })
}

/// Percentage change from `before` to `now`, in tenths of a percent.
/// Returns `None` when `before` is zero.
fn change(now: i64, before: i64) -> Option<i64> {
    (before != 0).then(|| (now - before) * 1_000 / before)
}

/// Converts tenths of a percent to an unsigned percentage, so `-42` becomes `4.2`.
#[expect(
    clippy::cast_precision_loss,
    reason = "changes stay far below 2^53, where the conversion is exact"
)]
fn percent(change: i64) -> f64 {
    change.unsigned_abs() as f64 / 10.0
}
