use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    l10n::{DateTime, DateTimeStyle, Translation},
    router::{
        content::Form,
        error::{RouterErrorExt, SeeOther, see_other},
        href, module_param, page, path_param, route,
    },
    runtime::{Event, link, signal},
    view::{View, attributes, component, element_id, suspense, view},
};

use super::{status_badge, t};
use crate::{
    app::{
        _admin::{
            Crumb,
            customers::customer_id::{self, CustomerId},
        },
        _common::money,
    },
    components::{
        avatar::{avatar, avatar_fallback},
        button::{ButtonVariant, button},
        card::{card, card_content, card_description, card_header, card_title},
        dialog::{
            dialog, dialog_content, dialog_description, dialog_footer, dialog_header, dialog_title,
        },
        skeleton::skeleton,
        table::{table, table_body, table_cell, table_footer, table_head, table_header, table_row},
    },
    features::{
        auth::require_user,
        catalog::Money,
        customer::Customer,
        order::{Order, OrderItem, OrderStatus},
    },
};

module_param!(pub order_id: u64 else not_found);

#[page(with(Crumb::new(async |cx| Ok(format!("#{}", path_param::<OrderId>(cx)?)))))]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    let id = *path_param::<OrderId>(cx)?;
    // Look up the order before streaming starts so a missing order can return 404.
    let order = Order::find(cx, id).await?.ok_or_not_found()?;
    let next = if user.role.can_edit() {
        order.status.next()
    } else {
        &[]
    };

    Ok(view! {
        <header class="flex flex-col gap-4 sm:flex-row sm:items-start">
            <div class="flex flex-col gap-1">
                <div class="flex items-center gap-3">
                    <h1 class="text-2xl font-semibold tracking-tight">
                        (t!(order_title, id: order.id.to_string()))
                    </h1>
                    status_badge(status: order.status)
                </div>
                <p class="text-sm text-muted-foreground">
                    <time datetime=(order.placed_at.to_string())>
                        (t!(placed_on, date: order.placed_at))
                    </time>
                </p>
            </div>
            if !next.is_empty() {
                status_actions(id: order.id, next: next)
            }
        </header>

        <div class="grid grid-cols-1 gap-6 lg:grid-cols-3 lg:items-start">
            card(
                attrs: attributes! { class="lg:col-span-2" },
                suspense(
                    fallback: view! { order_items_skeleton() },
                    order_items(id: order.id, total: order.total)
                )
            )

            card(
                card_header(card_title((t!(customer))))
                suspense(
                    fallback: view! { customer_details_skeleton() },
                    customer_details(id: order.customer_id)
                )
            )
        </div>
    })
}

#[component]
async fn order_items(cx: &Cx, id: u64, total: Money) -> Result<impl View> {
    let items = OrderItem::of_order(cx, id).await?;
    let units = items.iter().map(|item| item.quantity).sum::<u32>();

    Ok(view! {
        card_header(
            card_title((t!(items)))
            card_description((t!(unit_count, count: units)))
        )
        table(
            order_item_columns()
            table_body(
                for item in &items {
                    let product = item.product.get();
                    table_row(
                        table_cell(
                            <div class="flex flex-col">
                                <span class="font-medium">(&product.name)</span>
                                <span class="text-xs text-muted-foreground">
                                    (&product.sku)
                                </span>
                            </div>
                        )
                        table_cell(
                            attrs: attributes! { class="text-right tabular-nums" },
                            (item.quantity)
                        )
                        table_cell(
                            attrs: attributes! { class="text-right text-muted-foreground tabular-nums" },
                            (money(item.unit_price))
                        )
                        table_cell(
                            attrs: attributes! { class="text-right font-medium tabular-nums" },
                            (money(item.subtotal()))
                        )
                    )
                }
            )
            table_footer(
                table_row(
                    table_cell(attrs: attributes! { colspan="3" }, (t!(total)))
                    table_cell(
                        attrs: attributes! { class="text-right tabular-nums" },
                        (money(total))
                    )
                )
            )
        )
    })
}

#[component]
async fn order_items_skeleton() -> Result<impl View> {
    Ok(view! {
        card_header(
            card_title((t!(items)))
            <div class="flex h-5 items-center">
                skeleton(attrs: attributes! { class="h-3.5 w-14" })
            </div>
        )
        table(
            attrs: attributes! { aria-busy="true" },
            order_item_columns()
            table_body(
                for _ in 0..3 {
                    table_row(
                        table_cell(
                            <div class="flex flex-col">
                                <div class="flex h-5 items-center">
                                    skeleton(
                                        attrs: attributes! { class="h-3.5 w-40 max-w-full" }
                                    )
                                </div>
                                <div class="flex h-4 items-center">
                                    skeleton(attrs: attributes! { class="h-3 w-20" })
                                </div>
                            </div>
                        )
                        table_cell(
                            skeleton(attrs: attributes! { class="ml-auto h-4 w-6" })
                        )
                        table_cell(
                            skeleton(attrs: attributes! { class="ml-auto h-4 w-14" })
                        )
                        table_cell(
                            skeleton(attrs: attributes! { class="ml-auto h-4 w-16" })
                        )
                    )
                }
            )
            table_footer(
                table_row(
                    table_cell(attrs: attributes! { colspan="3" }, (t!(total)))
                    table_cell(
                        skeleton(attrs: attributes! { class="ml-auto h-4 w-16" })
                    )
                )
            )
        )
    })
}

#[component]
async fn order_item_columns() -> Result<impl View> {
    Ok(view! {
        table_header(
            table_row(
                table_head(attrs: attributes! { class="w-full" }, (t!(column_product)))
                table_head(
                    attrs: attributes! { class="text-right" },
                    (t!(column_quantity))
                )
                table_head(
                    attrs: attributes! { class="text-right" },
                    (t!(column_price))
                )
                table_head(
                    attrs: attributes! { class="text-right" },
                    (t!(column_subtotal))
                )
            )
        )
    })
}

#[component]
async fn customer_details(cx: &Cx, id: u64) -> Result<impl View> {
    let customer = Customer::get(cx, id).await?;

    Ok(view! {
        card_content(
            <div class="flex items-center gap-3">
                avatar(avatar_fallback((customer.initials())))
                <div class="flex min-w-0 flex-col">
                    link(
                        href: href!(customer_id::page, CustomerId(customer.id)),
                        attrs: attributes! { class="truncate font-medium hover:underline" },
                        (&customer.name)
                    )
                    <a
                        href=(format!("mailto:{}", customer.email))
                        class="truncate text-sm text-muted-foreground hover:underline"
                    >
                        (&customer.email)
                    </a>
                </div>
            </div>
            <dl class="mt-6 grid grid-cols-2 gap-4 text-sm">
                <div class="flex flex-col gap-1">
                    <dt class="text-muted-foreground">(t!(city))</dt>
                    <dd>(&customer.city)</dd>
                </div>
                <div class="flex flex-col gap-1">
                    <dt class="text-muted-foreground">(t!(customer_since))</dt>
                    <dd>
                        <time datetime=(customer.joined_at.to_string())>
                            (DateTime::from(customer.joined_at).date_style(
                                DateTimeStyle::Medium,
                            ))
                        </time>
                    </dd>
                </div>
            </dl>
        )
    })
}

#[component]
async fn customer_details_skeleton() -> Result<impl View> {
    Ok(view! {
        card_content(
            attrs: attributes! { aria-busy="true" },
            <div class="flex items-center gap-3">
                avatar(skeleton(attrs: attributes! { class="size-full" }))
                <div class="flex min-w-0 flex-1 flex-col">
                    <div class="flex h-6 items-center">
                        skeleton(attrs: attributes! { class="h-4 w-32 max-w-full" })
                    </div>
                    <div class="flex h-5 items-center">
                        skeleton(attrs: attributes! { class="h-3.5 w-44 max-w-full" })
                    </div>
                </div>
            </div>
            <dl class="mt-6 grid grid-cols-2 gap-4 text-sm">
                <div class="flex flex-col gap-1">
                    <dt class="text-muted-foreground">(t!(city))</dt>
                    <dd class="flex h-5 items-center">
                        skeleton(attrs: attributes! { class="h-3.5 w-20" })
                    </dd>
                </div>
                <div class="flex flex-col gap-1">
                    <dt class="text-muted-foreground">(t!(customer_since))</dt>
                    <dd class="flex h-5 items-center">
                        skeleton(attrs: attributes! { class="h-3.5 w-16" })
                    </dd>
                </div>
            </dl>
        )
    })
}

#[component]
async fn status_actions(cx: &Cx, id: u64, next: &'static [OrderStatus]) -> Result<impl View> {
    let confirming = signal(cx, || false);
    let form_id = element_id(cx);
    let dialog_id = element_id(cx);
    let title_id = element_id(cx);
    let reversal = next.iter().copied().find(|status| status.is_reversal());

    Ok(view! {
        <form
            id=(&form_id)
            method="post"
            action=(href!(advance, OrderId(id)))
            class="flex gap-2 sm:ml-auto"
        >
            for &status in next {
                if status.is_reversal() {
                    button(
                        variant: ButtonVariant::Outline,
                        attrs: attributes! {
                            type="button"
                            aria-haspopup="dialog"
                            aria-controls=(&dialog_id)
                            @click=$(|_e: Event| confirming.set(true))
                        },
                        (status_action(status))
                    )
                } else {
                    button(
                        attrs: attributes! { type="submit" name="status" value=(status.slug()) },
                        (status_action(status))
                    )
                }
            }
        </form>

        if let Some(status) = reversal {
            dialog(
                open: $(confirming.get()),
                attrs: attributes! {
                    id=(&dialog_id)
                    aria-labelledby=(&title_id)
                    @keydown=$(|e: Event| {
                        if e.key == "Escape" {
                            confirming.set(false);
                        }
                    })
                    @click=$(|e: Event| {
                        // Backdrop clicks target the dialog; clicks inside it target its children.
                        if e.target.id == dialog_id {
                            confirming.set(false);
                        }
                    })
                },
                dialog_content(
                    dialog_header(
                        dialog_title(
                            attrs: attributes! { id=(&title_id) },
                            (t!(confirm_status, status: status.slug()))
                        )
                        dialog_description((t!(final_status, id: id.to_string())))
                    )
                    dialog_footer(
                        button(
                            variant: ButtonVariant::Outline,
                            attrs: attributes! {
                                type="button"
                                @click=$(|_e: Event| confirming.set(false))
                            },
                            (t!(keep_order))
                        )
                        button(
                            variant: ButtonVariant::Destructive,
                            attrs: attributes! {
                                type="submit"
                                form=(&form_id)
                                name="status"
                                value=(status.slug())
                            },
                            (status_action(status))
                        )
                    )
                )
            )
        }
    })
}

/// The label of a button that moves an order to `status`.
fn status_action(status: OrderStatus) -> Translation<'static> {
    match status {
        OrderStatus::Pending => t!(action_pending),
        OrderStatus::Paid => t!(action_paid),
        OrderStatus::Shipped => t!(action_shipped),
        OrderStatus::Delivered => t!(action_delivered),
        OrderStatus::Cancelled => t!(action_cancelled),
        OrderStatus::Refunded => t!(action_refunded),
    }
}

#[derive(Deserialize)]
pub struct Advance {
    status: OrderStatus,
}

#[route(POST)]
pub async fn advance(cx: &Cx, Form(form): Form<Advance>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    user.role.can_edit().then_some(()).ok_or_forbidden()?;

    let id = *path_param::<OrderId>(cx)?;
    let mut order = Order::find(cx, id).await?.ok_or_not_found()?;
    // Another request may have changed the status since this page was opened.
    // If this transition is no longer allowed, show the current order unchanged.
    order.advance(cx, form.status).await?;

    Ok(see_other(href!(page, OrderId(id)).resolve(cx)))
}
