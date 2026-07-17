use std::collections::HashMap;

use serde::Deserialize;

/// Number of products rendered per `/products` page.
pub const PAGE_SIZE: usize = 24;

#[derive(Deserialize)]
pub struct Category {
    pub name: String,
    pub slug: String,
}

#[derive(Deserialize)]
pub struct Product {
    pub id: u32,
    pub name: String,
    pub category: String,
    pub category_slug: String,
    pub price_cents: u32,
    pub rating_tenths: u32,
    pub review_count: u32,
    pub featured: bool,
    pub specs: Vec<Spec>,
    pub description: Vec<String>,
    pub reviews: Vec<Review>,
    pub related_ids: Vec<u32>,
}

impl Product {
    /// Price formatted as `$12.99`; plain integer math so every benchmark app
    /// formats identically.
    pub fn price(&self) -> String {
        format!("${}.{:02}", self.price_cents / 100, self.price_cents % 100)
    }

    /// First letter of the first two words of the name, used as the image
    /// placeholder.
    pub fn initials(&self) -> String {
        self.name
            .split_whitespace()
            .take(2)
            .filter_map(|word| word.chars().next())
            .collect()
    }
}

#[derive(Deserialize)]
pub struct Spec {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct Review {
    pub author: String,
    pub date: String,
    pub rating_tenths: u32,
    pub title: String,
    pub body: String,
}

/// A rating in tenths formatted as `4.3`.
pub fn format_rating(tenths: u32) -> String {
    format!("{}.{}", tenths / 10, tenths % 10)
}

/// Number of filled stars out of five for a rating in tenths, rounded half-up.
pub fn filled_stars(tenths: u32) -> u32 {
    (tenths + 5) / 10
}

/// The product catalog, deserialized once at startup from the shared
/// `benchmarks/data/products.json` and shared with the handlers as axum state.
pub struct Catalog {
    categories: Vec<Category>,
    products: Vec<Product>,
    by_id: HashMap<u32, usize>,
}

impl Catalog {
    pub fn load() -> Self {
        #[derive(Deserialize)]
        struct Data {
            categories: Vec<Category>,
            products: Vec<Product>,
        }

        let data: Data = serde_json::from_str(include_str!("../../data/products.json"))
            .expect("products.json matches the catalog schema");
        let by_id = data
            .products
            .iter()
            .enumerate()
            .map(|(index, product)| (product.id, index))
            .collect();

        Self {
            categories: data.categories,
            products: data.products,
            by_id,
        }
    }

    pub fn categories(&self) -> &[Category] {
        &self.categories
    }

    pub fn get(&self, id: u32) -> Option<&Product> {
        self.by_id.get(&id).map(|&index| &self.products[index])
    }

    pub fn featured(&self) -> impl Iterator<Item = &Product> {
        self.products.iter().filter(|product| product.featured)
    }

    /// One page of the (filtered, sorted) product list.
    ///
    /// The semantics are the parity contract shared by every benchmark app:
    /// `category` filters by exact slug match (unknown slugs match nothing),
    /// `sort` must already be normalized to one of `name`, `price`,
    /// `price-desc`, or `rating` (anything else keeps ascending id order), all
    /// sorts tie-break by ascending id, and `page` is clamped to the valid
    /// range.
    pub fn page(
        &self,
        page: usize,
        sort: Option<&str>,
        category: Option<&str>,
    ) -> ProductsPage<'_> {
        let mut items: Vec<&Product> = self
            .products
            .iter()
            .filter(|product| category.is_none_or(|slug| product.category_slug == slug))
            .collect();

        match sort {
            Some("name") => items.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id))),
            Some("price") => {
                items.sort_by(|a, b| a.price_cents.cmp(&b.price_cents).then(a.id.cmp(&b.id)));
            }
            Some("price-desc") => {
                items.sort_by(|a, b| b.price_cents.cmp(&a.price_cents).then(a.id.cmp(&b.id)));
            }
            Some("rating") => {
                items.sort_by(|a, b| b.rating_tenths.cmp(&a.rating_tenths).then(a.id.cmp(&b.id)));
            }
            _ => {}
        }

        let total = items.len();
        let page_count = total.div_ceil(PAGE_SIZE).max(1);
        let current = page.clamp(1, page_count);
        let start = (current - 1) * PAGE_SIZE;
        items.drain(..start.min(total));
        items.truncate(PAGE_SIZE);

        ProductsPage {
            items,
            current,
            page_count,
            total,
        }
    }
}

pub struct ProductsPage<'a> {
    pub items: Vec<&'a Product>,
    pub current: usize,
    pub page_count: usize,
    pub total: usize,
}
