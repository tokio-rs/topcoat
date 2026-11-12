use serde::Deserialize;

/// A product category from `categories.json`.
#[derive(Debug, Deserialize)]
pub struct Category {
    pub slug: String,
    pub name: String,
    pub description: String,
    /// Number of products in the category, counted at startup.
    #[serde(skip)]
    pub product_count: usize,
}
