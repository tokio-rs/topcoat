use serde::{Deserialize, Serialize};

/// A product category from `categories.json`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Category {
    pub slug: String,
    pub name: String,
    pub description: String,
    /// Number of products in the category, counted at startup.
    #[serde(default)]
    pub product_count: usize,
}
