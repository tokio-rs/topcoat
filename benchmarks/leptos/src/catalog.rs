//! Server-only product catalog, loaded once from the shared
//! `benchmarks/data/products.json`.

use std::{collections::HashMap, sync::LazyLock};

use serde::Deserialize;

use crate::model::{CategoryData, ProductDetailData, ProductSummary, ReviewData, SpecData};

/// Number of products rendered per `/products` page.
pub const PAGE_SIZE: usize = 24;

pub static CATALOG: LazyLock<Catalog> = LazyLock::new(Catalog::load);

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
    pub fn summary(&self) -> ProductSummary {
        ProductSummary {
            id: self.id,
            name: self.name.clone(),
            category: self.category.clone(),
            price_cents: self.price_cents,
            rating_tenths: self.rating_tenths,
            review_count: self.review_count,
        }
    }

    pub fn detail(&self, catalog: &Catalog) -> ProductDetailData {
        ProductDetailData {
            id: self.id,
            name: self.name.clone(),
            category: self.category.clone(),
            category_slug: self.category_slug.clone(),
            price_cents: self.price_cents,
            rating_tenths: self.rating_tenths,
            review_count: self.review_count,
            specs: self
                .specs
                .iter()
                .map(|spec| SpecData {
                    key: spec.key.clone(),
                    value: spec.value.clone(),
                })
                .collect(),
            description: self.description.clone(),
            reviews: self
                .reviews
                .iter()
                .map(|review| ReviewData {
                    author: review.author.clone(),
                    date: review.date.clone(),
                    rating_tenths: review.rating_tenths,
                    title: review.title.clone(),
                    body: review.body.clone(),
                })
                .collect(),
            related: self
                .related_ids
                .iter()
                .filter_map(|id| catalog.get(*id))
                .map(Product::summary)
                .collect(),
        }
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

pub struct Catalog {
    categories: Vec<Category>,
    products: Vec<Product>,
    by_id: HashMap<u32, usize>,
}

impl Catalog {
    fn load() -> Self {
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

    pub fn categories(&self) -> Vec<CategoryData> {
        self.categories
            .iter()
            .map(|category| CategoryData {
                name: category.name.clone(),
                slug: category.slug.clone(),
            })
            .collect()
    }

    pub fn get(&self, id: u32) -> Option<&Product> {
        self.by_id.get(&id).map(|&index| &self.products[index])
    }

    pub fn featured(&self) -> impl Iterator<Item = &Product> {
        self.products.iter().filter(|product| product.featured)
    }

    /// One page of the (filtered, sorted) product list; same semantics as the
    /// other benchmark apps (see `benchmarks/topcoat/src/catalog.rs`).
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
