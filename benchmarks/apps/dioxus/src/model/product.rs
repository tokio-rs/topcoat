use serde::{Deserialize, Serialize};
use time::Date;

use super::iso_date;

/// A product from `products.json`, with every field.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
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
}

impl Product {
    /// The most a customer can add to the cart at once.
    pub fn max_quantity(&self) -> u32 {
        self.stock.min(10)
    }

    /// The rating in tenths of a star, for example `43` for a 4.3 rating.
    pub fn rating_tenths(&self) -> u32 {
        rating_tenths(self.rating)
    }

    /// The description split into its paragraphs.
    pub fn paragraphs(&self) -> impl Iterator<Item = &str> {
        self.description.split("\n\n")
    }

    /// The fields a product card shows.
    #[cfg(feature = "server")]
    pub fn card(&self, category: &super::Category) -> ProductCard {
        ProductCard {
            id: self.id,
            name: self.name.clone(),
            category_name: category.name.clone(),
            tagline: self.tagline.clone(),
            price_cents: self.price_cents,
            compare_at_cents: self.compare_at_cents,
            rating: self.rating,
            review_count: self.review_count,
            stock: self.stock,
            color: self.color,
        }
    }
}

/// The part of a product shown in a product grid.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ProductCard {
    pub id: u32,
    pub name: String,
    pub category_name: String,
    pub tagline: String,
    pub price_cents: u32,
    pub compare_at_cents: Option<u32>,
    pub rating: f64,
    pub review_count: u32,
    pub stock: u32,
    pub color: Color,
}

impl ProductCard {
    /// The rating in tenths of a star, for example `43` for a 4.3 rating.
    pub fn rating_tenths(&self) -> u32 {
        rating_tenths(self.rating)
    }
}

/// One row of a product's specification table.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Spec {
    pub label: String,
    pub value: String,
}

/// A customer review of a product.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Review {
    pub author: String,
    pub rating: u32,
    pub title: String,
    pub body: String,
    #[serde(with = "iso_date")]
    pub date: Date,
}

/// The color swatch of a product.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
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

fn rating_tenths(rating: f64) -> u32 {
    (rating * 10.0).round() as u32
}
