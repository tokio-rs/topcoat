use std::collections::BTreeMap;

use topcoat::runtime::record;
use topcoat_runtime_coherence::{Awaitable, Observe, Value, coherent};

#[record]
#[derive(Clone)]
struct Item {
    name: String,
    count: u32,
}

impl Observe for Item {
    fn observe(&self) -> Value {
        Value::Record(BTreeMap::from([
            ("name".to_owned(), self.name.observe()),
            ("count".to_owned(), self.count.observe()),
        ]))
    }
}

#[record]
#[derive(Clone)]
struct Order {
    r#type: String,
    items: Vec<Item>,
    note: Option<Item>,
}

impl Observe for Order {
    fn observe(&self) -> Value {
        Value::Record(BTreeMap::from([
            ("type".to_owned(), self.r#type.observe()),
            ("items".to_owned(), self.items.observe()),
            ("note".to_owned(), self.note.observe()),
        ]))
    }
}

fn order() -> Order {
    Order {
        r#type: "pickup".to_owned(),
        items: vec![
            Item {
                name: "coffee".to_owned(),
                count: 2,
            },
            Item {
                name: String::new(),
                count: u32::MAX,
            },
        ],
        note: None,
    }
}

#[test]
fn captured_records() {
    let order = order();
    coherent!(order);
    coherent!(order.clone());
    coherent!(order.r#type);
    coherent!(order.r#type.len());
    coherent!(order.items.len());
    coherent!(order.note.is_none());
    coherent!(direct => order.items);
    coherent!(async => order);
}

#[test]
fn borrowed_records() {
    let order = order();
    coherent!(*order.items.index(0).count);
    coherent!(order.items.index(1).name.is_empty());
    coherent!(order.items.last().unwrap().clone());
    coherent!(order.items.first().unwrap().name.to_owned());
    coherent!(*order.items.index(1).count + 1u32);
}

#[test]
fn constructed_records() {
    let name = String::from("tea");
    coherent!(Item {
        count: 1u32,
        name: name.clone(),
    });
    coherent!(
        Item {
            name: name.clone(),
            count: 3u32
        }
        .count
    );

    let order = order();
    coherent!(Order {
        r#type: order.r#type.clone(),
        items: order.items.clone(),
        note: Some(order.items.index(0).clone()),
    });
    coherent!(
        Order {
            r#type: "".to_owned(),
            items: order.items.to_vec(),
            note: None,
        }
        .items
        .len()
    );

    let item = Awaitable::ready(Item {
        name: String::from("late"),
        count: 4,
    })
    .after_yield();
    coherent!(async => {
        let item = item.await;
        Order { r#type: item.name.clone(), items: order.items.clone(), note: Some(item) }
    });
}

#[test]
fn updated_records() {
    let order = order();
    let item = order.items[0].clone();
    coherent!(Item {
        count: 5u32,
        ..item.clone()
    });
    coherent!(Item { ..item.clone() });
    coherent!(
        Item {
            name: "milk".to_owned(),
            ..item.clone()
        }
        .count
    );
    coherent!(Order {
        r#type: "".to_owned(),
        ..order.clone()
    });
    coherent!(Order {
        note: Some(Item {
            count: 0u32,
            ..order.items.index(1).clone()
        }),
        ..order.clone()
    });
    coherent!({
        let base = order.clone();
        Order {
            items: base.items.clone(),
            ..base
        }
    });
    // A panic in a field or in the base agrees in both languages.
    let missing: Option<Item> = None;
    coherent!(Order {
        note: Some(missing.unwrap()),
        ..order.clone()
    });
    coherent!(Item {
        count: 1u32,
        ..order.note.unwrap()
    });

    let late = Awaitable::ready(item.clone()).after_yield();
    coherent!(async => Item { count: 9u32, ..late.await });
}
