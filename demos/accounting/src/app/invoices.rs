pub mod detail;
pub mod new;

use topcoat::{
    Result,
    context::Cx,
    router::{href, page},
    runtime::{Event, shard, signal},
    view::{View, attributes, component, view},
};

use crate::{
    components::{
        badge::{BadgeVariant, badge},
        button::{ButtonSize, ButtonVariant, button, button_variants},
        input::input,
        select::select,
        table::{table, table_body, table_cell, table_head, table_header, table_row},
    },
    models::{Invoice, invoices, money},
};

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let query = signal(cx, String::new);
    let status = signal(cx, || "All".to_owned());
    Ok(view! {
        <div class="page-heading">
            <div><h1>"Invoices"</h1></div>
            <a
                href=(href!(new::page))
                class=(button_variants(ButtonVariant::Primary, ButtonSize::Md))
            >
                "+ New invoice"
            </a>
        </div>
        <section>
            <div class="mb-6 flex flex-wrap gap-3">
                input(
                    attrs: attributes! {
                        type="search"
                        aria-label="Search invoices"
                        placeholder="Search customer or invoice..."
                        class="max-w-sm"
                        :value=$(query.get())
                        @input=$(|e: Event| query.set(e.target.value))
                    }
                )
                select(
                    attrs: attributes! {
                        class="w-40 max-w-full shrink-0"
                        aria-label="Invoice status"
                        :value=$(status.get())
                        @change=$(|e: Event| status.set(e.target.value))
                    },
                    <option>"All"</option>
                    <option>"Outstanding"</option>
                    <option>"Overdue"</option>
                    <option>"Paid"</option>
                )
                button(
                    variant: ButtonVariant::Ghost,
                    attrs: attributes! {
                        type="button"
                        @click=$(|_e: Event| {
                            query.set("".to_owned());
                            status.set("All".to_owned());
                        })
                    },
                    "Reset"
                )
            </div>
            results(query: $(query.get()), status: $(status.get()))
        </section>
    })
}

#[shard]
async fn results(cx: &Cx, query: String, status: String) -> Result<impl View> {
    let needle = query.trim().to_lowercase();
    let invoices: Vec<_> = invoices(cx)
        .await?
        .into_iter()
        .filter(|invoice| {
            (invoice.customer.to_lowercase().contains(&needle)
                || invoice.id.to_lowercase().contains(&needle))
                && (status == "All" || invoice.status() == status)
        })
        .collect();
    Ok(view! {
        invoice_table(invoices: &invoices)
        <p class="mt-5 text-xs text-muted-foreground" role="status">
            (invoices.len())
            " invoices"
        </p>
    })
}

#[component]
pub async fn invoice_table(invoices: &[Invoice]) -> Result<impl View> {
    Ok(view! {
        table(
            table_header(
                table_row(
                    table_head("Invoice / Customer")
                    table_head("Due date")
                    table_head("Status")
                    table_head(attrs: attributes! { class="text-right" }, "Amount")
                )
            )
            table_body(
                #[key(invoice.id.clone())]
                for invoice in invoices {
                    table_row(
                        attrs: attributes! { id=(format!("invoice-{}", invoice.id)) },
                        table_cell(
                            <a
                                class="font-medium text-foreground hover:underline"
                                href=(href!(detail::page, detail::InvoiceId(&invoice.id)))
                            >
                                (number(&invoice.id))
                            </a>
                            <p class="mt-1 text-xs text-muted-foreground">
                                (&invoice.customer)
                            </p>
                        )
                        table_cell((&invoice.due))
                        table_cell(status_badge(status: invoice.status()))
                        table_cell(
                            attrs: attributes! { class="text-right font-medium tabular-nums" },
                            (money(invoice.total()))
                        )
                    )
                }
            )
        )
        if invoices.is_empty() {
            <div class="py-16 text-center">
                <h3 class="font-medium">"No invoices found"</h3>
                <p class="mt-2 text-sm text-muted-foreground">
                    "Try a different search or create your first invoice."
                </p>
            </div>
        }
    })
}

pub fn number(id: &str) -> String {
    if id.starts_with("INV-") {
        id.to_owned()
    } else {
        format!("INV-{}", id.get(..8).unwrap_or(id).to_uppercase())
    }
}

#[component]
pub async fn status_badge(status: &'static str) -> Result<impl View> {
    let color = match status {
        "Paid" => "bg-teal-50 text-teal-800 border-teal-100",
        "Overdue" => "bg-rose-50 text-rose-700 border-rose-100",
        _ => "bg-amber-50 text-amber-800 border-amber-100",
    };
    Ok(view! {
        badge(
            variant: BadgeVariant::Outline,
            attrs: attributes! { class=(color) },
            (status)
        )
    })
}
