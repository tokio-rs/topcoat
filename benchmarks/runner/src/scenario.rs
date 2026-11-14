//! The benchmarked scenarios from `SPEC.md`.

use serde::Serialize;

use crate::dataset::Dataset;

/// Every scenario id, in display order.
pub const SCENARIO_IDS: [&str; 9] = [
    "plaintext",
    "about",
    "home",
    "listing",
    "detail",
    "orders",
    "api-list",
    "api-detail",
    "api-cart",
];

/// What a scenario's responses contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Body {
    Text,
    Html,
    Json,
}

/// One benchmarked request pattern. Requests rotate randomly over `paths` and `bodies`.
#[derive(Debug, Clone, Serialize)]
pub struct Scenario {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub method: &'static str,
    pub response: Body,
    pub paths: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bodies: Vec<String>,
}

const LISTING_QUERIES: [&str; 2] = ["?category=outdoor&sort=price-asc&page=1", "?page=3"];

impl Scenario {
    /// Builds every scenario for `dataset`.
    pub fn all(dataset: &Dataset) -> Vec<Self> {
        let get = |id, title, description, response, paths: Vec<String>| Self {
            id,
            title,
            description,
            method: "GET",
            response,
            paths,
            bodies: Vec::new(),
        };
        let ids = &dataset.product_ids;
        vec![
            get(
                "plaintext",
                "Plaintext",
                "GET /health, the framework floor",
                Body::Text,
                vec!["/health".to_owned()],
            ),
            get(
                "about",
                "About page",
                "Small, mostly static page in the shared layout",
                Body::Html,
                vec!["/about".to_owned()],
            ),
            get(
                "home",
                "Home page",
                "Hero, category links and 8 featured product cards",
                Body::Html,
                vec!["/".to_owned()],
            ),
            get(
                "listing",
                "Product listing",
                "24 product cards, category sidebar, sort links and pagination",
                Body::Html,
                LISTING_QUERIES
                    .iter()
                    .map(|q| format!("/products{q}"))
                    .collect(),
            ),
            get(
                "detail",
                "Product detail",
                "Product, spec table, reviews, related products, rotating over every product",
                Body::Html,
                ids.iter().map(|id| format!("/products/{id}")).collect(),
            ),
            get(
                "orders",
                "Orders table",
                "Large page: a table of 500 orders with formatted money and dates",
                Body::Html,
                vec!["/admin/orders".to_owned()],
            ),
            get(
                "api-list",
                "API: product list",
                "JSON for the listing query",
                Body::Json,
                LISTING_QUERIES
                    .iter()
                    .map(|q| format!("/api/products{q}"))
                    .collect(),
            ),
            get(
                "api-detail",
                "API: product detail",
                "Full product JSON, rotating over every product",
                Body::Json,
                ids.iter().map(|id| format!("/api/products/{id}")).collect(),
            ),
            Self {
                id: "api-cart",
                title: "API: add to cart",
                description: "POST JSON, validate against the catalog, respond with the cart line",
                method: "POST",
                response: Body::Json,
                paths: vec!["/api/cart".to_owned()],
                bodies: dataset.cart_bodies.clone(),
            },
        ]
    }
}
