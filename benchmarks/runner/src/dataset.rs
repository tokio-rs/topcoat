//! The parts of the shared dataset the runner needs to build requests.

use std::path::Path;

use serde::Deserialize;

/// Product ids and request bodies derived from `data/products.json`.
#[derive(Debug, Clone)]
pub struct Dataset {
    /// Every product id, in file order.
    pub product_ids: Vec<u32>,
    /// Valid `POST /api/cart` bodies.
    pub cart_bodies: Vec<String>,
    /// A product that is in stock, used for the detail page payload measurement.
    pub in_stock: u32,
    /// A sold out product, if the dataset has one.
    pub sold_out: Option<u32>,
    /// A product on sale, if the dataset has one.
    pub on_sale: Option<u32>,
    /// Every category slug.
    pub categories: Vec<String>,
}

#[derive(Deserialize)]
struct Product {
    id: u32,
    #[serde(default)]
    stock: Option<u32>,
    #[serde(default)]
    compare_at_cents: Option<u64>,
    #[serde(default)]
    category: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Products {
    List(Vec<Product>),
    Wrapped { products: Vec<Product> },
}

impl Dataset {
    /// Reads `products.json` from `data_dir`. Falls back to ids `1..=240` with a warning when
    /// the file is missing or unreadable.
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("products.json");
        let parsed = std::fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|text| {
                serde_json::from_str::<Products>(&text).map_err(|error| error.to_string())
            });
        match parsed {
            Ok(Products::List(products) | Products::Wrapped { products })
                if !products.is_empty() =>
            {
                Self::from_products(&products)
            }
            Ok(_) => Self::fallback(&format!("{} has no products", path.display())),
            Err(error) => Self::fallback(&format!("cannot read {}: {error}", path.display())),
        }
    }

    fn from_products(products: &[Product]) -> Self {
        let mut cart_bodies = Vec::new();
        for product in products {
            let limit = product.stock.unwrap_or(1).min(10);
            for quantity in 1..=limit.min(3) {
                cart_bodies.push(format!(
                    r#"{{"product_id":{},"quantity":{quantity}}}"#,
                    product.id
                ));
            }
        }
        let mut categories: Vec<String> = products
            .iter()
            .filter_map(|product| product.category.clone())
            .collect();
        categories.sort();
        categories.dedup();
        Self {
            product_ids: products.iter().map(|product| product.id).collect(),
            cart_bodies,
            in_stock: products
                .iter()
                .find(|product| product.stock.unwrap_or(1) > 0)
                .map_or(1, |product| product.id),
            sold_out: products
                .iter()
                .find(|product| product.stock == Some(0))
                .map(|product| product.id),
            on_sale: products
                .iter()
                .find(|product| product.compare_at_cents.is_some())
                .map(|product| product.id),
            categories,
        }
    }

    fn fallback(reason: &str) -> Self {
        eprintln!(
            "{} {reason}; assuming product ids 1..=240",
            console::style("warning:").yellow().bold()
        );
        Self {
            product_ids: (1..=240).collect(),
            cart_bodies: vec![r#"{"product_id":1,"quantity":1}"#.to_owned()],
            in_stock: 1,
            sold_out: None,
            on_sale: None,
            categories: vec!["outdoor".to_owned()],
        }
    }
}
