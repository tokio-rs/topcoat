use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::{Event, shard, signal},
    view::{View, attributes, component, view},
};

use crate::{
    components::{
        button::{ButtonSize, ButtonVariant, button_variants},
        card::{card, card_content},
        select::select,
    },
    models::{invoices, money},
};

#[shard]
pub async fn overview(cx: &Cx) -> Result<impl View> {
    let period = signal(cx, || "all".to_owned());
    let selected = period.get();

    let cutoff =
        crate::models::date(time::OffsetDateTime::now_utc().date() - time::Duration::days(30));
    let invoices: Vec<_> = invoices(cx)
        .await?
        .into_iter()
        .filter(|invoice| selected != "month" || invoice.issued >= cutoff)
        .collect();
    let billed: i64 = invoices.iter().map(crate::models::Invoice::total).sum();
    let paid: i64 = invoices
        .iter()
        .filter(|invoice| invoice.paid)
        .map(crate::models::Invoice::total)
        .sum();
    let overdue: i64 = invoices
        .iter()
        .filter(|invoice| invoice.status() == "Overdue")
        .map(crate::models::Invoice::total)
        .sum();
    let collection = if billed == 0 { 0 } else { paid * 100 / billed };

    Ok(view! {
        <div class="page-heading">
            <div><h1>"Overview"</h1></div>
            <a
                href=(href!(super::invoices::new::page))
                class=(button_variants(ButtonVariant::Primary, ButtonSize::Md))
            >
                "+ New invoice"
            </a>
        </div>
        <div class="mb-4 flex flex-wrap items-center justify-between gap-3">
            <h2 class="text-sm font-medium">"Financial summary"</h2>
            select(
                attrs: attributes! {
                    aria-label="Reporting period"
                    class="w-60 max-w-full shrink-0"
                    :value=$(period.get())
                    @change=$(|e: Event| period.set(e.target.value))
                },
                <option value="all">"All invoices"</option>
                <option value="month">"Issued in the last 30 days"</option>
            )
        </div>
        <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            metric(
                title: "Total invoiced",
                value: money(billed),
                note: format!("{} invoices issued", invoices.len())
            )
            metric(
                title: "Collected",
                value: money(paid),
                note: "Paid invoices".to_owned()
            )
            metric(
                title: "Outstanding",
                value: money(billed - paid),
                note: "Awaiting payment".to_owned()
            )
            metric(
                title: "Overdue",
                value: money(overdue),
                note: "Past their due date".to_owned()
            )
        </div>
        <div class="mt-8 grid items-start gap-6 lg:grid-cols-3">
            card(
                attrs: attributes! { class="min-w-0 lg:col-span-2" },
                card_content(
                    <section>
                        <div class="mb-6 flex items-center justify-between">
                            <h2 class="text-sm font-semibold">"Recent invoices"</h2>
                            <a
                                class="text-xs text-muted-foreground hover:text-foreground hover:underline"
                                href=(href!(super::invoices::page))
                            >
                                "View all invoices"
                            </a>
                        </div>
                        super::invoices::invoice_table(
                            invoices: &invoices[..invoices.len().min(5)]
                        )
                    </section>
                )
            )
            card(
                attrs: attributes! { class="min-w-0" },
                card_content(
                    <section>
                        <h2 class="text-sm font-semibold">"Collection progress"</h2>
                        <p
                            class="mt-6 text-4xl font-semibold tracking-tight tabular-nums"
                        >
                            (collection)
                            "%"
                        </p>
                        <p class="mt-2 text-sm text-muted-foreground">
                            "of invoiced value has been paid"
                        </p>
                        <div
                            class="my-5 h-1.5 overflow-hidden rounded-full bg-zinc-100"
                            role="progressbar"
                            aria-label="Invoice collection"
                            aria-valuenow=(collection)
                            aria-valuemin="0"
                            aria-valuemax="100"
                        >
                            <div
                                class="h-full rounded-full bg-primary"
                                style=(format!("width: {collection}%"))
                            ></div>
                        </div>
                        <dl class="space-y-4 text-sm">
                            <div class="flex justify-between">
                                <dt>"Collected"</dt>
                                <dd class="font-medium">(money(paid))</dd>
                            </div>
                            <div class="flex justify-between">
                                <dt>"Still to collect"</dt>
                                <dd class="font-medium">(money(billed - paid))</dd>
                            </div>
                        </dl>
                    </section>
                )
            )
        </div>
    })
}

#[component]
async fn metric(title: &'static str, value: String, note: String) -> Result<impl View> {
    Ok(view! {
        card(
            card_content(
                <p class="text-xs font-medium text-muted-foreground">(title)</p>
                <p class="mt-2 text-2xl font-semibold tracking-tight tabular-nums">
                    (value)
                </p>
                <p class="mt-2 text-xs text-muted-foreground">(note)</p>
            )
        )
    })
}
