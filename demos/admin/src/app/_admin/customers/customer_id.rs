use topcoat::{
    Result,
    context::{Cx, memoize},
    icon::{IconData, icon, iconify::iconify_icon},
    router::{
        Body, Next, error::RouterErrorExt, href, layer, module_param, page, path_param,
        response::Response,
    },
    runtime::link,
    view::{Child, View, attributes, component, suspense, view},
};

use crate::{
    app::_admin::{
        Crumb,
        orders::{
            order_id::{self, OrderId},
            status_badge, time_ago,
        },
    },
    components::{
        avatar::{AvatarSize, avatar, avatar_fallback},
        card::{card, card_description, card_header, card_title},
        skeleton::skeleton,
        table::{table, table_body, table_cell, table_head, table_header, table_row},
    },
    features::{
        auth::require_user,
        customer::{Customer, Spending},
        order::Order,
    },
};

module_param!(pub customer_id: u64, error = not_found);

const SKELETON_ROWS: usize = 5;

#[memoize(as_ref)]
async fn customer(cx: &Cx) -> Result<Customer> {
    let id = *path_param::<CustomerId>(cx)?;
    Ok(Customer::find(cx, id).await?.ok_or_not_found()?)
}

#[layer]
async fn crumb(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let name = customer(cx).await?.name.clone();
    let cx = cx.with(Crumb(name));
    next.run(&cx, body).await
}

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let customer = customer(cx).await?;

    Ok(view! {
        <header class="flex items-center gap-4">
            avatar(size: AvatarSize::Lg, avatar_fallback((customer.initials())))
            <div class="flex min-w-0 flex-col gap-1">
                <h1 class="truncate text-2xl font-semibold tracking-tight">
                    (&customer.name)
                </h1>
                <div
                    class="flex flex-wrap gap-x-4 gap-y-1 text-sm text-muted-foreground"
                >
                    detail(
                        data: iconify_icon!("lucide:mail"),
                        <a
                            href=(format!("mailto:{}", customer.email))
                            class="hover:underline"
                        >
                            (&customer.email)
                        </a>
                    )
                    detail(data: iconify_icon!("lucide:map-pin"), (&customer.city))
                    detail(
                        data: iconify_icon!("lucide:calendar"),
                        "Customer since "
                        <time datetime=(customer.joined_at.to_string())>
                            (customer.joined_at.strftime("%b %Y").to_string())
                        </time>
                    )
                </div>
            </div>
        </header>

        suspense(
            fallback: view! { customer_orders_skeleton() },
            customer_orders(id: customer.id)
        )
    })
}

#[component]
async fn detail(data: IconData, child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <span class="flex items-center gap-1.5">
            icon(data: data, attrs: attributes! { class="size-4" })
            (child)
        </span>
    })
}

#[component]
async fn customer_orders(cx: &Cx, id: u64) -> Result<impl View> {
    let orders = Order::of_customer(cx, id).await?;
    let spending = Spending::of(&orders);

    Ok(view! {
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
            stat(label: "Orders", (spending.orders))
            stat(label: "Total spent", (spending.spent))
            stat(
                label: "Average order",
                match spending.average_order() {
                    Some(average) => (average),
                    None => "-",
                }
            )
        </div>

        card(
            card_header(
                card_title("Orders")
                card_description("Cancelled and refunded orders do not count as spent.")
            )
            if orders.is_empty() {
                <p class="px-6 text-sm text-muted-foreground">
                    "This customer has not placed an order yet."
                </p>
            } else {
                table(
                    order_columns()
                    table_body(
                        for order in &orders {
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
            }
        )
    })
}

#[component]
async fn customer_orders_skeleton() -> Result<impl View> {
    Ok(view! {
        <div aria-busy="true" class="grid grid-cols-1 gap-4 sm:grid-cols-3">
            for label in ["Orders", "Total spent", "Average order"] {
                stat(
                    label: label,
                    <div class="flex h-8 items-center">
                        skeleton(attrs: attributes! { class="h-6 w-20" })
                    </div>
                )
            }
        </div>

        card(
            card_header(
                card_title("Orders")
                card_description("Cancelled and refunded orders do not count as spent.")
            )
            table(
                attrs: attributes! { aria-busy="true" },
                order_columns()
                table_body(
                    for _ in 0..SKELETON_ROWS {
                        table_row(
                            table_cell(
                                skeleton(attrs: attributes! { class="h-4 w-10" })
                            )
                            table_cell(
                                skeleton(attrs: attributes! { class="h-5.5 w-20" })
                            )
                            table_cell(
                                skeleton(attrs: attributes! { class="h-4 w-14" })
                            )
                            table_cell(
                                skeleton(attrs: attributes! { class="h-4 w-20" })
                            )
                            table_cell(
                                skeleton(attrs: attributes! { class="ml-auto h-4 w-16" })
                            )
                        )
                    }
                )
            )
        )
    })
}

#[component]
async fn stat(label: &str, child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        card(
            attrs: attributes! { class="gap-1" },
            <p class="px-6 text-sm text-muted-foreground">(label)</p>
            <div class="px-6 text-2xl font-semibold tracking-tight tabular-nums">
                (child)
            </div>
        )
    })
}

#[component]
async fn order_columns() -> Result<impl View> {
    Ok(view! {
        table_header(
            table_row(
                table_head("Order")
                table_head("Status")
                table_head("Items")
                table_head(attrs: attributes! { class="w-full" }, "Placed")
                table_head(attrs: attributes! { class="text-right" }, "Total")
            )
        )
    })
}
