use serde::{Deserialize, Serialize};
use time::Date;

time::serde::format_description!(iso_date, Date, "[year]-[month]-[day]");

/// A product from `products.json`.
#[derive(Debug, Deserialize, Serialize)]
pub struct Product {
    pub id: u32,
    pub slug: String,
    pub name: String,
    pub category: String,
    pub price_cents: u32,
    pub compare_at_cents: Option<u32>,
    pub rating: f64,
    pub review_count: u32,
    pub stock: u32,
    pub color: Color,
    pub tagline: String,
    pub description: String,
    pub specs: Vec<Spec>,
    #[serde(with = "iso_date")]
    pub created_at: Date,
    pub featured: bool,
    pub related: Vec<u32>,
    pub reviews: Vec<Review>,
    /// Position of the product's category in the store, resolved at startup.
    #[serde(skip)]
    pub category_index: usize,
}

impl Product {
    /// The most a customer can add to the cart at once.
    pub fn max_quantity(&self) -> u32 {
        self.stock.min(10)
    }

    pub fn is_sold_out(&self) -> bool {
        self.stock == 0
    }

    /// The rating in tenths of a star, for example `43` for a 4.3 rating.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "ratings are between 1.0 and 5.0"
    )]
    pub fn rating_tenths(&self) -> u32 {
        (self.rating * 10.0).round() as u32
    }

    /// The description split into its paragraphs.
    pub fn paragraphs(&self) -> impl Iterator<Item = &str> {
        self.description.split("\n\n")
    }
}

/// The product fields included in list responses.
#[derive(Serialize)]
pub struct ProductSummary<'a> {
    pub id: u32,
    pub slug: &'a str,
    pub name: &'a str,
    pub category: &'a str,
    pub price_cents: u32,
    pub compare_at_cents: Option<u32>,
    pub rating: f64,
    pub review_count: u32,
    pub stock: u32,
    pub color: Color,
    pub tagline: &'a str,
    #[serde(with = "iso_date")]
    pub created_at: Date,
    pub featured: bool,
}

impl<'a> From<&'a Product> for ProductSummary<'a> {
    fn from(product: &'a Product) -> Self {
        Self {
            id: product.id,
            slug: &product.slug,
            name: &product.name,
            category: &product.category,
            price_cents: product.price_cents,
            compare_at_cents: product.compare_at_cents,
            rating: product.rating,
            review_count: product.review_count,
            stock: product.stock,
            color: product.color,
            tagline: &product.tagline,
            created_at: product.created_at,
            featured: product.featured,
        }
    }
}

/// One row of a product's specification table.
#[derive(Debug, Deserialize, Serialize)]
pub struct Spec {
    pub label: String,
    pub value: String,
}

/// A customer review of a product.
#[derive(Debug, Deserialize, Serialize)]
pub struct Review {
    pub author: String,
    pub rating: u32,
    pub title: String,
    pub body: String,
    #[serde(with = "iso_date")]
    pub date: Date,
}

/// The color swatch of a product.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Color {
    Charcoal,
    Sand,
    Sage,
    Navy,
    Terracotta,
    Ochre,
    Plum,
    Sky,
}

impl Color {
    pub fn label(self) -> &'static str {
        match self {
            Self::Charcoal => "Charcoal",
            Self::Sand => "Sand",
            Self::Sage => "Sage",
            Self::Navy => "Navy",
            Self::Terracotta => "Terracotta",
            Self::Ochre => "Ochre",
            Self::Plum => "Plum",
            Self::Sky => "Sky",
        }
    }
}
