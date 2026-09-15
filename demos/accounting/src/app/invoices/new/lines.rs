use std::collections::HashMap;

use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, Signal, procedure, shard, signal},
    view::{View, attributes, view},
};

use crate::{
    components::{
        button::{ButtonSize, ButtonVariant, button},
        card::{card, card_content},
        input::input,
        label::label,
    },
    draft::{Line, MAX_LINES},
    models::{money, tax as invoice_tax},
};

// An ordinary server-side tuple. Only scalar vectors cross the shard boundary.
type InitialLine = (usize, String, String, String);

pub fn initial(form: &HashMap<String, String>) -> Vec<InitialLine> {
    let keys = form.get("line-keys").map_or("0", String::as_str);
    let mut seen = std::collections::HashSet::new();
    let lines: Vec<_> = keys
        .split(',')
        .filter_map(|key| key.parse::<usize>().ok())
        .filter(|key| *key < 1_000_000 && seen.insert(*key))
        .take(MAX_LINES)
        .map(|key| {
            let value = |name: &str, default: &str| {
                form.get(&format!("{name}-{key}"))
                    .cloned()
                    .unwrap_or_else(|| default.to_owned())
            };
            (
                key,
                value("description", ""),
                value("quantity", "1"),
                value("price", "0.00"),
            )
        })
        .collect();
    if lines.is_empty() {
        vec![(0, String::new(), "1".to_owned(), "0.00".to_owned())]
    } else {
        lines
    }
}

struct LineInputs {
    key: usize,
    description: Signal<String>,
    quantity: Signal<String>,
    price: Signal<String>,
    total: std::result::Result<i64, String>,
}

impl LineInputs {
    fn new(cx: &Cx, key: usize, initial: &[InitialLine]) -> Self {
        let value = initial.iter().find(|line| line.0 == key);
        let description = signal(cx, || value.map_or_else(String::new, |line| line.1.clone()));
        let quantity = signal(cx, || {
            value.map_or_else(|| "1".to_owned(), |line| line.2.clone())
        });
        let price = signal(cx, || {
            value.map_or_else(|| "0.00".to_owned(), |line| line.3.clone())
        });
        // These reads subscribe the surrounding shard, not the whole form.
        let total =
            Line::parse(String::new(), &quantity.get(), &price.get()).map(|line| line.total());
        Self {
            key,
            description,
            quantity,
            price,
            total,
        }
    }
}

#[shard]
pub async fn line_items(
    cx: &Cx,
    initial_keys: Vec<usize>,
    descriptions: Vec<String>,
    quantities: Vec<String>,
    prices: Vec<String>,
    tax: Signal<String>,
    busy: Signal<bool>,
) -> Result<impl View> {
    let initial: Vec<_> = initial_keys
        .into_iter()
        .zip(descriptions)
        .zip(quantities)
        .zip(prices)
        .take(MAX_LINES)
        .map(|(((key, description), quantity), price)| (key, description, quantity, price))
        .collect();
    let keys = signal(cx, || initial.iter().map(|line| line.0).collect::<Vec<_>>());
    let next_key = signal(cx, || {
        initial
            .iter()
            .map(|line| line.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    });
    let current = keys.get();
    let mut seen = std::collections::HashSet::new();
    let rows: Vec<_> = current
        .iter()
        .copied()
        .filter(|key| *key < 1_000_000 && seen.insert(*key))
        .take(MAX_LINES)
        .map(|key| LineInputs::new(&cx.keyed(key), key, &initial))
        .collect();
    let subtotal = rows.iter().try_fold(0, |sum, row| {
        row.total.as_ref().ok().map(|amount| sum + amount)
    });
    let percent = tax
        .get()
        .parse::<i64>()
        .ok()
        .filter(|value| (0..=100).contains(value));
    let key_list = rows
        .iter()
        .map(|row| row.key.to_string())
        .collect::<Vec<_>>()
        .join(",");
    Ok(view! {
        card(
            card_content(
                <section>
                    <div class="mb-6 flex items-center justify-between">
                        <h2 class="text-sm font-semibold">"Line items"</h2>
                        <span class="text-xs text-muted-foreground">
                            (rows.len())
                            " / 20 lines"
                        </span>
                    </div>
                    <input type="hidden" name="line-keys" value=(key_list)>
                    <div class="space-y-4">
                        #[key(row.key)]
                        for row in &rows {
                            let key = row.key;
                            let description = &row.description;
                            let quantity = &row.quantity;
                            let price = &row.price;
                            <div id=(format!("line-{key}")) class="line-row">
                                <div class="space-y-2">
                                    label(
                                        attrs: attributes! { for=(format!("description-{key}")) },
                                        "Description"
                                    )
                                    input(
                                        attrs: attributes! {
                                            id=(format!("description-{key}"))
                                            name=(format!("description-{key}"))
                                            placeholder="Service or product"
                                            required=""
                                            maxlength="300"
                                            :value=$(description.get())
                                            @input=$(|e: Event| description.set(e.target.value))
                                        }
                                    )
                                </div>
                                <div class="space-y-2">
                                    label(
                                        attrs: attributes! { for=(format!("quantity-{key}")) },
                                        "Qty"
                                    )
                                    input(
                                        attrs: attributes! {
                                            id=(format!("quantity-{key}"))
                                            name=(format!("quantity-{key}"))
                                            type="number"
                                            min="1"
                                            max="10000"
                                            step="1"
                                            required=""
                                            :value=$(quantity.get())
                                            @input=$(|e: Event| quantity.set(e.target.value))
                                        }
                                    )
                                </div>
                                <div class="space-y-2">
                                    label(
                                        attrs: attributes! { for=(format!("price-{key}")) },
                                        "Unit price ($)"
                                    )
                                    input(
                                        attrs: attributes! {
                                            id=(format!("price-{key}"))
                                            name=(format!("price-{key}"))
                                            type="number"
                                            min="0"
                                            max="1000000"
                                            step="0.01"
                                            required=""
                                            :value=$(price.get())
                                            @input=$(|e: Event| price.set(e.target.value))
                                        }
                                    )
                                </div>
                                <div
                                    class="pb-2 text-right text-sm font-medium tabular-nums"
                                >
                                    (row
                                        .total
                                        .as_ref()
                                        .map_or_else(
                                            |_| "Check values".to_owned(),
                                            |total| money(*total),
                                        ))
                                </div>
                                button(
                                    variant: ButtonVariant::Ghost,
                                    size: ButtonSize::Sm,
                                    attrs: attributes! {
                                        type="button"
                                        aria-label=(format!("Remove line {}", key + 1))
                                        :disabled=$(if busy.get() {
                                            true
                                        } else {
                                            keys.get().len() <= 1
                                        })
                                        @click=$(async move |_e: Event| {
                                            busy.set(true);
                                            let updated = change_lines(keys.get(), key, true).await;
                                            keys.set(updated);
                                            busy.set(false);
                                        })
                                    },
                                    "Remove"
                                )
                            </div>
                        }
                    </div>
                    <div
                        class="mt-6 flex flex-wrap items-start justify-between gap-8 border-t border-border pt-6"
                    >
                        button(
                            variant: ButtonVariant::Outline,
                            attrs: attributes! {
                                type="button"
                                :disabled=$(if busy.get() {
                                    true
                                } else {
                                    keys.get().len() >= 20
                                })
                                @click=$(async move |_e: Event| {
                                    busy.set(true);
                                    let updated = change_lines(
                                        keys.get(),
                                        next_key.get(),
                                        false,
                                    ).await;
                                    next_key.increment();
                                    keys.set(updated);
                                    busy.set(false);
                                })
                            },
                            "+ Add line"
                        )
                        <div class="w-full max-w-xs" aria-live="polite">
                            if let (Some(subtotal), Some(percent)) = (subtotal, percent) {
                                <dl class="space-y-3 text-sm">
                                    <div class="flex justify-between">
                                        <dt>"Subtotal"</dt>
                                        <dd>(money(subtotal))</dd>
                                    </div>
                                    <div class="flex justify-between">
                                        <dt>
                                            "Tax ("
                                            (percent)
                                            "%)"
                                        </dt>
                                        <dd>(money(invoice_tax(subtotal, percent)))</dd>
                                    </div>
                                    <div
                                        class="flex justify-between border-t border-border pt-3 text-lg font-semibold"
                                    >
                                        <dt>"Total USD"</dt>
                                        <dd>(money(subtotal + invoice_tax(subtotal, percent)))</dd>
                                    </div>
                                </dl>
                            } else {
                                <p class="text-sm text-rose-700">
                                    "Check quantities, prices, and tax to see the total."
                                </p>
                            }
                        </div>
                    </div>
                </section>
            )
        )
    })
}

// Vec reads work in expressions; push/retain do not. Keep this temporary
// round-trip small and explicit instead of hiding it behind raw JavaScript.
#[procedure]
async fn change_lines(mut keys: Vec<usize>, key: usize, remove: bool) -> Result<Vec<usize>> {
    let mut seen = std::collections::HashSet::new();
    keys.retain(|key| *key < 1_000_000 && seen.insert(*key));
    keys.truncate(MAX_LINES);
    if remove && keys.len() > 1 {
        keys.retain(|value| *value != key);
    } else if !remove && keys.len() < MAX_LINES && key < 1_000_000 && !keys.contains(&key) {
        keys.push(key);
    }
    Ok(keys)
}
