//! Tests for using `#[record]` structs in runtime expressions.

use serde_json::json;
use topcoat::{
    Result,
    runtime::{Expr, Surrogate, Surrogated, expr, procedure, record},
};

#[record]
#[derive(Debug, Clone, PartialEq)]
struct Item {
    name: String,
    count: u32,
}

#[record]
#[derive(Debug, Clone, PartialEq)]
struct Order {
    id: u64,
    r#type: String,
    items: Vec<Item>,
    parent: Option<Vec<Self>>,
}

fn order() -> Order {
    Order {
        id: 7,
        r#type: "pickup".to_owned(),
        items: vec![Item {
            name: "coffee".to_owned(),
            count: 2,
        }],
        parent: None,
    }
}

#[test]
fn owned_and_borrowed_records_serialize_their_fields() {
    let order = order();
    let item = json!({ "t": "Record", "v": {
        "name": "coffee",
        "count": { "t": "u32", "bits": 32, "v": "2" },
    }});
    let wire = json!({ "t": "Record", "v": {
        "id": { "t": "u64", "bits": 64, "v": "7" },
        "type": "pickup",
        "items": { "t": "Vec", "bits": usize::BITS, "v": [item] },
        "parent": { "t": "Option", "v": null },
    }});

    assert_eq!(
        serde_json::to_value((&order).into_surrogate()).unwrap(),
        wire
    );
    assert_eq!(
        serde_json::to_value(order.clone().into_surrogate()).unwrap(),
        wire
    );

    let decoded: <Order as Surrogated>::Surrogate = serde_json::from_value(wire).unwrap();
    assert_eq!(decoded.into_real(), order);
}

#[test]
fn deserialization_rejects_malformed_records() {
    let count = json!({ "t": "u32", "bits": 32, "v": "2" });
    for wire in [
        json!({ "name": "coffee", "count": count }),
        json!({ "t": "Tuple", "v": { "name": "coffee", "count": count } }),
        json!({ "t": "Record", "v": { "name": "coffee" } }),
        json!({ "t": "Record", "v": { "name": "coffee", "count": count, "extra": true } }),
        json!({ "t": "Record", "v": { "name": 1, "count": count } }),
    ] {
        assert!(serde_json::from_value::<<Item as Surrogated>::Surrogate>(wire).is_err());
    }
}

#[test]
fn expressions_construct_records_and_read_fields() {
    let name = String::from("tea");
    let (item, js) = expr!(Item { count: 3u32, name }).into_evaluated_and_js();
    assert_eq!(
        item,
        Item {
            name: "tea".to_owned(),
            count: 3
        }
    );
    assert!(js.to_source().contains("cx.record({count: "));

    let order = order();
    let (kind, js) = expr!(order.r#type.len()).into_evaluated_and_js();
    assert_eq!(kind, "pickup".len());
    assert!(js.to_source().contains(".type.len()"));

    let (count, _) = expr!(*order.items.index(0).count).into_evaluated_and_js();
    assert_eq!(count, 2);

    let (copy, _) = expr!(order.items.first().unwrap().clone()).into_evaluated_and_js();
    assert_eq!(copy, order.items[0]);

    let (nested, _) = expr!(
        Order {
            id: 1u64,
            r#type: order.r#type.clone(),
            items: order.items.clone(),
            parent: None,
        }
        .items
        .len()
    )
    .into_evaluated_and_js();
    assert_eq!(nested, 1);
}

#[test]
fn expressions_update_records_from_a_base() {
    let order = order();
    let (updated, _) = expr!(Order {
        id: 8u64,
        ..order.clone()
    })
    .into_evaluated_and_js();
    assert_eq!(updated.id, 8);
    assert_eq!(updated.r#type, order.r#type);
    assert_eq!(updated.items, order.items);

    let (item, _) = expr!(Item {
        ..order.items.index(0).clone()
    })
    .into_evaluated_and_js();
    assert_eq!(item, order.items[0]);
}

/// Exercises record operations that do not require `Clone`.
#[record]
struct Plain {
    label: String,
}

#[test]
fn records_without_clone_are_usable() {
    let label = String::from("hi");
    let (plain, _) = expr!(Plain { label }).into_evaluated_and_js();
    assert_eq!(plain.label, "hi");
    let (plain, _) = Expr::from(plain).into_evaluated_and_js();
    assert_eq!(plain.label, "hi");
}

#[procedure]
async fn total(order: Order) -> Result<Item> {
    Ok(Item {
        name: order.r#type,
        count: order.items.iter().map(|item| item.count).sum(),
    })
}

#[test]
fn procedures_accept_and_return_records() {
    let order = order();
    let (_, js) = expr!(async || total(order).await.count).into_evaluated_and_js();
    assert!(js.to_source().contains(".count"));
}
