use std::sync::atomic::{AtomicU64, Ordering};

use topcoat::{
    Result,
    context::Cx,
    router::page,
    runtime::{Event, procedure, record, signal},
    view::{View, view},
};

// Declare an order that expressions can construct, read, store in a signal,
// and submit to a server procedure.
#[record]
#[derive(Clone)]
pub struct Order {
    item: String,
    quantity: u32,
    address: Address,
}

// Use a nested record to group the address fields within an order.
#[record]
#[derive(Clone)]
pub struct Address {
    street: String,
    city: String,
}

#[record]
#[derive(Clone)]
pub struct Receipt {
    number: u64,
    summary: String,
}

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let order = signal(cx, || Order {
        item: "coffee".to_owned(),
        quantity: 1,
        address: Address {
            street: String::new(),
            city: String::new(),
        },
    });
    let receipt = signal(cx, || None::<Result<Receipt, String>>);

    // This server read subscribes the page to receipt changes. A new receipt
    // from the browser triggers another render of the Rust match below.
    let placed = receipt.get();

    Ok(view! {
        // Update a field by replacing the signal's order with a new record.
        <label>
            "item "
            <input
                :value=$(order.read().item.to_owned())
                @input=$(|e: Event| {
                    let current = order.get();
                    order.set(
                        Order {
                            item: e.target.value,
                            quantity: current.quantity,
                            address: current.address,
                        },
                    );
                })
            >
        </label>

        <p>
            "quantity: "
            $(order.get().quantity)
            " "
            <button
                @click=$(|_e| {
                    let current = order.get();
                    if current.quantity > 1u32 {
                        order.set(
                            Order {
                                quantity: current.quantity - 1u32,
                                item: current.item,
                                address: current.address,
                            },
                        );
                    }
                })
            >
                "-"
            </button>
            <button
                @click=$(|_e| {
                    let current = order.get();
                    order.set(
                        Order {
                            quantity: current.quantity + 1u32,
                            item: current.item,
                            address: current.address,
                        },
                    );
                })
            >
                "+"
            </button>
        </p>

        <label>
            "street "
            <input
                :value=$(order.read().address.street.to_owned())
                @input=$(|e: Event| {
                    let current = order.get();
                    order.set(
                        Order {
                            address: Address {
                                street: e.target.value,
                                city: current.address.city,
                            },
                            item: current.item,
                            quantity: current.quantity,
                        },
                    );
                })
            >
        </label>

        <label>
            "city "
            <input
                :value=$(order.read().address.city.to_owned())
                @input=$(|e: Event| {
                    let current = order.get();
                    order.set(
                        Order {
                            address: Address {
                                street: current.address.street,
                                city: e.target.value,
                            },
                            item: current.item,
                            quantity: current.quantity,
                        },
                    );
                })
            >
        </label>

        // Submit the complete order and save the procedure's result.
        <p>
            <button
                @click=$(async |_e| {
                    let result = place_order(order.get()).await;
                    receipt.set(Some(result));
                })
            >
                "place order"
            </button>
        </p>

        match placed {
            Some(Ok(receipt)) => {
                <p>
                    "Order #"
                    (receipt.number)
                    ": "
                    (receipt.summary)
                </p>
            }
            Some(Err(message)) => <p>
                "Could not place the order: "
                (message)
            </p>,
            None => {

            }
        }
    })
}

static NEXT_ORDER: AtomicU64 = AtomicU64::new(1);

// Validate the browser's order before accepting it. Return validation errors
// in the inner `Result` so the page can display them.
#[procedure]
pub async fn place_order(order: Order) -> Result<Result<Receipt, String>> {
    if !(1..=10).contains(&order.quantity) {
        return Ok(Err("orders are limited to 10 items".to_owned()));
    }
    if order.address.street.trim().is_empty() || order.address.city.trim().is_empty() {
        return Ok(Err("please enter a street and city".to_owned()));
    }

    Ok(Ok(Receipt {
        number: NEXT_ORDER.fetch_add(1, Ordering::Relaxed),
        summary: format!(
            "{} x {} to {}, {}",
            order.quantity, order.item, order.address.street, order.address.city
        ),
    }))
}
