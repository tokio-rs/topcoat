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
            <div>
                <p class="eyebrow">"YOUR BUSINESS AT A GLANCE"</p>
                <h1>"Overview"</h1>
                <p class="mt-2 text-muted-foreground">
                    "A little clarity for your working day."
                </p>
            </div>
            <a
                href=(href!(super::invoices::new::page))
                class=(button_variants(ButtonVariant::Primary, ButtonSize::Md))
            >
                "+ New invoice"
            </a>
        </div>
        <div class="mb-6 flex items-center justify-between gap-4">
            <h2 class="text-lg font-semibold">"Financial snapshot"</h2>
            <select
                aria-label="Reporting period"
                class="select-control w-auto"
                :value=$(period.get())
                @change=$(|e: Event| period.set(e.target.value))
            >
                <option value="all">"All invoices"</option>
                <option value="month">"Issued in the last 30 days"</option>
            </select>
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
        <div class="mt-8 grid gap-6 lg:grid-cols-3">
            <section class="panel lg:col-span-2">
                <div class="mb-6 flex items-center justify-between">
                    <h2 class="text-lg font-semibold">"Recent invoices"</h2>
                    <a
                        class="text-sm font-medium text-teal-700"
                        href=(href!(super::invoices::page))
                    >
                        "View all invoices"
                    </a>
                </div>
                super::invoices::invoice_table(
                    invoices: &invoices[..invoices.len().min(5)]
                )
            </section>
            <section class="panel">
                <h2 class="text-lg font-semibold">"Collection progress"</h2>
                <p class="mt-8 text-5xl font-semibold tracking-tight">
                    (collection)
                    "%"
                </p>
                <p class="mt-2 text-sm text-muted-foreground">
                    "of invoiced value has been paid"
                </p>
                <div
                    class="my-6 h-3 overflow-hidden rounded-full bg-stone-100"
                    role="progressbar"
                    aria-label="Invoice collection"
                    aria-valuenow=(collection)
                    aria-valuemin="0"
                    aria-valuemax="100"
                >
                    <div
                        class="h-full rounded-full bg-teal-600"
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
        </div>
    })
}

#[component]
async fn metric(title: &'static str, value: String, note: String) -> Result<impl View> {
    Ok(view! {
        card(
            attrs: attributes! { class="bg-white" },
            card_content(
                <p class="text-sm text-muted-foreground">(title)</p>
                <p class="mt-3 text-3xl font-semibold tracking-tight">(value)</p>
                <p class="mt-3 text-xs text-muted-foreground">(note)</p>
            )
        )
    })
}
