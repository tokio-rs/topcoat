mod lines;

use std::collections::HashMap;

use topcoat::{
    Result,
    context::Cx,
    router::{StatusCode, content::Form, error::see_other, href, page},
    runtime::{Event, Signal, signal},
    view::{View, attributes, component, view},
};

use crate::{
    components::{
        button::{ButtonSize, ButtonVariant, button, button_variants},
        input::input,
        label::label,
        textarea::textarea,
    },
    draft::Draft,
    models::{date, db, today},
};

#[page]
pub async fn page() -> Result<impl View> {
    let form = HashMap::new();
    Ok(view! { editor(form: &form, error: None) })
}

// Native form submission collects every named field, including dynamic lines.
// This is the boundary to revisit when runtime records and collection writes land.
#[page(POST)]
async fn create(cx: &Cx, Form(form): Form<HashMap<String, String>>) -> Result<impl View> {
    match Draft::from_form(&form) {
        Ok(draft) => {
            let id = draft.save(&mut db(cx)).await?;
            Err(
                see_other(href!(super::detail::page, super::detail::InvoiceId(&id)).resolve(cx))
                    .into(),
            )
        }
        Err(error) => Ok(view! {
            (StatusCode::UNPROCESSABLE_ENTITY)
            editor(form: &form, error: Some(error))
        }),
    }
}

#[component]
async fn editor(
    cx: &Cx,
    form: &HashMap<String, String>,
    error: Option<String>,
) -> Result<impl View> {
    let value = |name: &str, default: &str| {
        form.get(name)
            .cloned()
            .unwrap_or_else(|| default.to_owned())
    };
    let customer = signal(cx, || value("customer", ""));
    let tax = signal(cx, || value("tax", "10"));
    let busy = signal(cx, || false);
    let initial = lines::initial(form);
    // Vec<(...)> cannot currently be captured by an expression. These four
    // vectors carry the initial field values to the shard in matching order.
    let initial_keys = initial.iter().map(|line| line.0).collect::<Vec<_>>();
    let descriptions = initial
        .iter()
        .map(|line| line.1.clone())
        .collect::<Vec<_>>();
    let quantities = initial
        .iter()
        .map(|line| line.2.clone())
        .collect::<Vec<_>>();
    let prices = initial
        .iter()
        .map(|line| line.3.clone())
        .collect::<Vec<_>>();
    let due = date(time::OffsetDateTime::now_utc().date() + time::Duration::days(30));
    Ok(view! {
        <a
            class="text-sm text-muted-foreground hover:underline"
            href=(href!(super::page))
        >
            "Back to invoices"
        </a>
        <div class="page-heading mt-5"><div><h1>"New invoice"</h1></div></div>
        if let Some(error) = error {
            <p
                role="alert"
                class="mb-6 rounded-xl border border-rose-200 bg-rose-50 p-4 text-sm text-rose-800"
            >
                (error)
            </p>
        }
        <form method="post" action=(href!(create)) class="space-y-6">
            <section class="panel">
                <h2 class="text-sm font-semibold">"Invoice details"</h2>
                <p class="mt-1 text-sm text-muted-foreground">
                    "From Studio Collective to "
                    $(if customer.get().is_empty() {
                        "your customer".to_owned()
                    } else {
                        customer.get()
                    })
                </p>
                <div class="mt-6 grid gap-5 sm:grid-cols-2">
                    text_field(
                        name: "customer",
                        title: "Customer / company",
                        kind: "text",
                        value: &customer,
                        required: true
                    )
                    field(
                        name: "email",
                        title: "Billing email",
                        kind: "email",
                        initial: value("email", ""),
                        required: true
                    )
                    field(
                        name: "issued",
                        title: "Issue date",
                        kind: "date",
                        initial: value("issued", &today()),
                        required: true
                    )
                    field(
                        name: "due",
                        title: "Due date",
                        kind: "date",
                        initial: value("due", &due),
                        required: true
                    )
                    <div class="space-y-2 sm:col-span-2">
                        label(attrs: attributes! { for="address" }, "Billing address")
                        textarea(
                            attrs: attributes! { id="address" name="address" rows="2" maxlength="1000" },
                            (value("address", ""))
                        )
                    </div>
                </div>
            </section>
            lines::line_items(
                initial_keys: $(initial_keys),
                descriptions: $(descriptions),
                quantities: $(quantities),
                prices: $(prices),
                tax: $(tax),
                busy: $(busy)
            )
            <section class="panel grid gap-6 sm:grid-cols-3">
                <div class="space-y-2 sm:col-span-2">
                    label(attrs: attributes! { for="notes" }, "Notes & payment terms")
                    textarea(
                        attrs: attributes! { id="notes" name="notes" rows="3" maxlength="2000" },
                        (value(
                            "notes",
                            "Thank you for your business. Payment is due within 30 days.",
                        ))
                    )
                </div>
                <div class="space-y-2">
                    label(attrs: attributes! { for="tax" }, "Tax (%)")
                    input(
                        attrs: attributes! {
                            id="tax"
                            name="tax"
                            type="number"
                            min="0"
                            max="100"
                            step="1"
                            required=""
                            :value=$(tax.get())
                            @input=$(|e: Event| tax.set(e.target.value))
                        }
                    )
                </div>
            </section>
            <div class="flex items-center justify-end gap-3">
                <a
                    href=(href!(super::page))
                    class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))
                >
                    "Cancel"
                </a>
                button(
                    attrs: attributes! { type="submit" :disabled=$(busy.get()) },
                    "Create invoice"
                )
            </div>
        </form>
    })
}

#[component]
async fn field(
    cx: &Cx,
    name: &'static str,
    title: &'static str,
    kind: &'static str,
    initial: String,
    required: bool,
) -> Result<impl View> {
    let value = signal(cx, || initial);
    Ok(view! {
        text_field(
            name: name,
            title: title,
            kind: kind,
            value: &value,
            required: required
        )
    })
}

#[component]
async fn text_field(
    name: &'static str,
    title: &'static str,
    kind: &'static str,
    value: &Signal<String>,
    required: bool,
) -> Result<impl View> {
    Ok(view! {
        <div class="space-y-2">
            label(attrs: attributes! { for=(name) }, (title))
            input(
                attrs: attributes! {
                    id=(name)
                    name=(name)
                    type=(kind)
                    required=(required)
                    maxlength="254"
                    :value=$(value.get())
                    @input=$(|e: Event| value.set(e.target.value))
                }
            )
        </div>
    })
}
