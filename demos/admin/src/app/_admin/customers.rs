pub(super) mod customer_id;

use serde::Serialize;
use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    l10n::{Translation, fluent::messages},
    router::{href, page, query_params},
    runtime::{Event, Signal, link, shard, signal},
    view::{View, attributes, component, element_id, suspense, view},
};

use self::customer_id::CustomerId;
use crate::{
    app::_common::{money, pagination_text},
    components::{
        avatar::{AvatarSize, avatar, avatar_fallback},
        button::{ButtonSize, ButtonVariant, button},
        input::input,
        list_pagination::{ListPage, list_pagination},
        select::select,
        skeleton::skeleton,
        table::{table, table_body, table_cell, table_head, table_header, table_row},
    },
    features::{
        auth::require_user,
        customer::{CustomerSearch, CustomerSort, Spending},
    },
};

messages!("{mod}/l10n/{locale}.ftl");

const PER_PAGE: usize = 20;

/// URL parameters for the initial list. Pagination clicks update a signal without
/// changing the URL; opening a page link in a new tab uses these parameters.
#[query_params(error = redirect("?"))]
#[derive(Serialize)]
pub struct CustomersQuery {
    q: Option<String>,
    sort: Option<CustomerSort>,
    page: Option<usize>,
}

impl CustomersQuery {
    fn new(text: &str, sort: CustomerSort, number: usize) -> Self {
        Self {
            q: (!text.is_empty()).then(|| text.to_owned()),
            sort: (sort != CustomerSort::default()).then_some(sort),
            page: (number > 1).then_some(number),
        }
    }
}

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    require_user(cx).await?;
    let query = query_params::<CustomersQuery>(cx)?;
    let initial_sort = query.sort.unwrap_or_default().slug();

    let search = signal(cx, || query.q.clone().unwrap_or_default());
    let sort = signal(cx, || initial_sort.to_owned());
    let page_number = signal(cx, || query.page.unwrap_or(1));
    let search_id = element_id(cx);
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
                <div class="sm:ml-auto">
                    <label for=(&sort_id) class="sr-only">(t!(sort))</label>
                    select(
                        attrs: attributes! {
                            id=(&sort_id)
                            class="sm:w-44"
                            :value=$(sort.get())
                            @change=$(|e: Event| {
                                sort.set(e.target.value);
                                page_number.set(1);
                            })
                        },
                        for option in CustomerSort::ALL {
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

            customer_list(
                search: search.clone(),
                sort: sort.clone(),
                page_number: page_number.clone()
            )
        </section>
    })
}

#[shard]
async fn customer_list(
    cx: &Cx,
    search: Signal<String>,
    sort: Signal<String>,
    page_number: Signal<usize>,
) -> Result<impl View> {
    // Shards can be requested without the page layout, so they need their own sign-in check.
    require_user(cx).await?;

    let text = search.get();
    let sort = sort.get().parse::<CustomerSort>().unwrap_or_default();
    let count = CustomerSearch { text: &text }.count(cx).await?;
    let shown = ListPage::new(count, PER_PAGE, page_number.get());

    Ok(view! {
        if count == 0 {
            <div class="flex flex-col items-center gap-3 px-6 py-16 text-center">
                <span
                    class="flex size-10 items-center justify-center rounded-full \
                        bg-foreground/5 text-muted-foreground"
                >
                    icon(data: iconify_icon!("lucide:user-search"))
                </span>
                <div class="flex flex-col gap-1">
                    <p class="font-medium">(t!(no-results))</p>
                    <p class="text-sm text-muted-foreground">(t!(no-results.description))</p>
                </div>
                button(
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    attrs: attributes! {
                        type="button"
                        @click=$(|_e: Event| {
                            search.set("".to_owned());
                            page_number.set(1);
                        })
                    },
                    (t!(clear-search))
                )
            </div>
        } else {
            suspense(
                fallback: view! { customer_rows_skeleton(rows: shown.items()) },
                customer_rows(
                    filter: CustomerSearch { text: &text },
                    sort: sort,
                    number: shown.number
                )
            )

            list_pagination(
                page: shown,
                text: pagination_text(shown),
                href: |number| {
                    href!(page)
                        .query(&CustomersQuery::new(&text, sort, number))
                        .resolve(cx)
                },
                number: &page_number
            )
        }
    })
}

#[component]
async fn customer_rows(
    cx: &Cx,
    filter: CustomerSearch<'_>,
    sort: CustomerSort,
    number: usize,
) -> Result<impl View> {
    let customers = filter.page(cx, sort, number, PER_PAGE).await?;

    Ok(view! {
        table(
            customer_columns()
            table_body(
                for customer in &customers {
                    let spending = Spending::of(customer.orders.get());
                    table_row(
                        attrs: attributes! { class="relative" },
                        table_cell(
                            <div class="flex items-center gap-3">
                                avatar(
                                    size: AvatarSize::Sm,
                                    avatar_fallback((customer.initials()))
                                )
                                <div class="flex max-w-64 min-w-0 flex-col">
                                    link(
                                        href: href!(customer_id::page, CustomerId(customer.id)),
                                        attrs: attributes! {
                                            class="truncate font-medium after:absolute after:inset-0"
                                        },
                                        (&customer.name)
                                    )
                                    <span class="truncate text-xs text-muted-foreground">
                                        (&customer.email)
                                    </span>
                                </div>
                            </div>
                        )
                        table_cell(
                            attrs: attributes! { class="text-muted-foreground" },
                            (&customer.city)
                        )
                        table_cell(
                            attrs: attributes! { class="text-muted-foreground" },
                            <time datetime=(customer.joined_at.to_string())>
                                (customer.joined_at.strftime("%b %-d, %Y").to_string())
                            </time>
                        )
                        table_cell(
                            attrs: attributes! { class="text-right text-muted-foreground tabular-nums" },
                            (spending.orders)
                        )
                        table_cell(
                            attrs: attributes! { class="text-right font-medium tabular-nums" },
                            (money(spending.spent))
                        )
                    )
                }
            )
        )
    })
}

#[component]
async fn customer_rows_skeleton(rows: usize) -> Result<impl View> {
    Ok(view! {
        table(
            attrs: attributes! { aria-busy="true" },
            customer_columns()
            table_body(
                for _ in 0..rows {
                    table_row(
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
                        table_cell(skeleton(attrs: attributes! { class="h-4 w-16" }))
                        table_cell(skeleton(attrs: attributes! { class="h-4 w-20" }))
                        table_cell(
                            skeleton(attrs: attributes! { class="ml-auto h-4 w-6" })
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
async fn customer_columns() -> Result<impl View> {
    Ok(view! {
        table_header(
            table_row(
                table_head(attrs: attributes! { class="w-full" }, (t!(column-customer)))
                table_head((t!(column-city)))
                table_head((t!(column-joined)))
                table_head(attrs: attributes! { class="text-right" }, (t!(column-orders)))
                table_head(attrs: attributes! { class="text-right" }, (t!(column-spent)))
            )
        )
    })
}

fn sort_label(sort: CustomerSort) -> Translation<'static> {
    match sort {
        CustomerSort::Name => t!(sort-name),
        CustomerSort::Newest => t!(sort-newest),
        CustomerSort::Oldest => t!(sort-oldest),
    }
}
