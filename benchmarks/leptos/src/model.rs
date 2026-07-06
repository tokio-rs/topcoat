//! Serializable DTOs shared between the server renderer and the hydrating
//! client. Pages receive these from server functions, so all rendering inputs
//! serialize into the document and hydration reproduces the exact markup.

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct CategoryData {
    pub name: String,
    pub slug: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProductSummary {
    pub id: u32,
    pub name: String,
    pub category: String,
    pub price_cents: u32,
    pub rating_tenths: u32,
    pub review_count: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SpecData {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ReviewData {
    pub author: String,
    pub date: String,
    pub rating_tenths: u32,
    pub title: String,
    pub body: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HomeData {
    pub featured: Vec<ProductSummary>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProductsData {
    pub categories: Vec<CategoryData>,
    pub items: Vec<ProductSummary>,
    pub current: usize,
    pub page_count: usize,
    pub total: usize,
    /// The normalized sort echoed back for links and chip highlighting.
    pub sort: Option<String>,
    /// The raw category parameter echoed back for links and chip highlighting.
    pub category: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProductDetailData {
    pub id: u32,
    pub name: String,
    pub category: String,
    pub category_slug: String,
    pub price_cents: u32,
    pub rating_tenths: u32,
    pub review_count: u32,
    pub specs: Vec<SpecData>,
    pub description: Vec<String>,
    pub reviews: Vec<ReviewData>,
    pub related: Vec<ProductSummary>,
}
