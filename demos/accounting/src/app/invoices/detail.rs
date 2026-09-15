use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, href, page, path_param},
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, view},
};

use crate::{
    components::{
        button::button,
        card::{card, card_content},
        table::{table, table_body, table_cell, table_head, table_header, table_row},
    },
    models::{Invoice, InvoiceLine, db, money, tax},
};

path_param!(pub invoice_id);

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let id = path_param::<InvoiceId>(cx).to_owned();
    Ok(view! { invoice_detail(id: $(id)) })
}

#[shard]
async fn invoice_detail(cx: &Cx, id: String) -> Result<impl View> {
    let revision = signal(cx, || 0usize);
    let busy = signal(cx, || false);
    let message = signal(cx, String::new);
    // A procedure changes the database, then this read refreshes the shard.
    let _ = revision.get();
    let invoice = Invoice::filter_by_id(&id)
        .first()
        .exec(&mut db(cx))
        .await?
        .ok_or_not_found()?;
    let lines = InvoiceLine::filter_by_invoice_id(&id)
        .order_by(InvoiceLine::fields().position().asc())
        .exec(&mut db(cx))
        .await?;
    Ok(view! {
        <a
            class="text-sm text-muted-foreground hover:underline"
            href=(href!(super::page))
        >
            "Back to invoices"
        </a>
        <div class="page-heading mt-5">
            <div><h1>(super::number(&id))</h1></div>
            if !invoice.paid {
                button(
                    attrs: attributes! {
                        type="button"
                        :disabled=$(busy.get())
                        @click=$(async move |_e: Event| {
                            busy.set(true);
                            let outcome = mark_paid(id.to_owned()).await;
                            busy.set(false);
                            if outcome.is_ok() {
                                message.set(outcome.unwrap());
                                revision.increment();
                            } else {
                                message.set(outcome.unwrap_err());
                            }
                        })
                    },
                    $(if busy.get() { "Saving..." } else { "Mark as paid" })
                )
            }
        </div>
        <p role="status" class="mb-4 text-sm" :hidden=$(message.get().is_empty())>
            $(message.get())
        </p>
        card(
            attrs: attributes! { class="mx-auto max-w-4xl" },
            card_content(
                <article class="sm:p-2 lg:p-6">
                    <div
                        class="flex flex-wrap items-start justify-between gap-8 border-b border-border pb-8"
                    >
                        <div>
                            <h2 class="text-2xl font-semibold">"Studio Collective"</h2>
                            <p class="mt-2 text-sm text-muted-foreground">
                                "Independent design & development"
                            </p>
                        </div>
                        super::status_badge(status: invoice.status())
                    </div>
                    <div class="my-8 grid gap-8 sm:grid-cols-2">
                        <div>
                            <p class="eyebrow">"Bill to"</p>
                            <h3 class="mt-3 font-semibold">(&invoice.customer)</h3>
                            <p class="mt-1 text-sm">(&invoice.email)</p>
                            <p
                                class="mt-2 whitespace-pre-line text-sm text-muted-foreground"
                            >
                                (&invoice.address)
                            </p>
                        </div>
                        <dl class="space-y-3 text-sm sm:text-right">
                            <div>
                                <dt class="text-muted-foreground">"Issued"</dt>
                                <dd class="mt-1">(&invoice.issued)</dd>
                            </div>
                            <div>
                                <dt class="text-muted-foreground">"Due date"</dt>
                                <dd class="mt-1">(&invoice.due)</dd>
                            </div>
                        </dl>
                    </div>
                    table(
                        table_header(
                            table_row(
                                table_head("Description")
                                table_head("Qty")
                                table_head("Unit price")
                                table_head(
                                    attrs: attributes! { class="text-right" },
                                    "Amount"
                                )
                            )
                        )
                        table_body(
                            #[key(line.id)]
                            for line in &lines {
                                table_row(
                                    table_cell((&line.description))
                                    table_cell((line.quantity))
                                    table_cell((money(line.unit_price)))
                                    table_cell(
                                        attrs: attributes! { class="text-right" },
                                        (money(line.quantity * line.unit_price))
                                    )
                                )
                            }
                        )
                    )
                    <dl class="ml-auto mt-8 max-w-xs space-y-3 text-sm">
                        <div class="flex justify-between">
                            <dt>"Subtotal"</dt>
                            <dd>(money(invoice.subtotal))</dd>
                        </div>
                        <div class="flex justify-between">
                            <dt>
                                "Tax ("
                                (invoice.tax_percent)
                                "%)"
                            </dt>
                            <dd>(money(tax(invoice.subtotal, invoice.tax_percent)))</dd>
                        </div>
                        <div
                            class="flex justify-between border-t border-border pt-4 text-lg font-semibold"
                        >
                            <dt>"Total USD"</dt>
                            <dd>(money(invoice.total()))</dd>
                        </div>
                    </dl>
                    <div class="mt-10 border-t border-border pt-6">
                        <h3 class="text-sm font-medium">"Notes & payment terms"</h3>
                        <p
                            class="mt-2 whitespace-pre-line text-sm text-muted-foreground"
                        >
                            (&invoice.notes)
                        </p>
                    </div>
                </article>
            )
        )
    })
}

#[procedure]
async fn mark_paid(cx: &Cx, id: String) -> Result<std::result::Result<String, String>> {
    // Return errors as data so the browser can show them and clear its busy state.
    let outcome: Result<()> = async {
        let mut db = db(cx);
        let mut invoice = Invoice::filter_by_id(&id)
            .first()
            .exec(&mut db)
            .await?
            .ok_or_not_found()?;
        toasty::update!(invoice { paid: true })
            .exec(&mut db)
            .await?;
        Ok(())
    }
    .await;
    Ok(outcome
        .map(|()| "Payment recorded.".to_owned())
        .map_err(|_| "Payment could not be recorded. Please try again.".to_owned()))
}
