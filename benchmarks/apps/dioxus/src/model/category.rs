use serde::{Deserialize, Serialize};

/// A product category from `categories.json`.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Category {
    pub slug: String,
    pub name: String,
    pub description: String,
    /// Number of products in the category, counted when the dataset is loaded.
    #[serde(default)]
    pub product_count: u32,
}
