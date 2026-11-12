//! The in-memory dataset.

mod listing;

pub use listing::*;

use std::{collections::HashMap, error::Error, fs, path::Path, sync::Arc};

use serde::de::DeserializeOwned;

use crate::model::{Category, Order, OrderSummary, Product, ProductSummary, Sort};

/// Product indexes sorted by each [`Sort`] order.
type SortedIndexes = [Vec<usize>; Sort::ALL.len()];

/// The in-memory dataset with the indexes built at startup.
///
/// Every lookup is a constant-time index or a slice, standing in for a database that already did
/// the filtering, sorting and aggregation. Shared values are reference counted so page data can
/// hold them without copying.
pub struct Store {
    categories: Arc<[Category]>,
    category_by_slug: HashMap<String, usize>,
    products: Vec<Arc<Product>>,
    sorted_all: SortedIndexes,
    sorted_by_category: Vec<SortedIndexes>,
    featured: Vec<usize>,
    orders: Arc<[Order]>,
    order_summary: OrderSummary,
}

impl Store {
    /// Loads the JSON dataset from `dir` and builds the indexes.
    pub fn load(dir: &Path) -> Result<Self, Box<dyn Error>> {
        let mut categories: Vec<Category> = read_json(&dir.join("categories.json"))?;
        let mut products: Vec<Product> = read_json(&dir.join("products.json"))?;
        let mut orders: Vec<Order> = read_json(&dir.join("orders.json"))?;

        let category_by_slug: HashMap<String, usize> = categories
            .iter()
            .enumerate()
            .map(|(index, category)| (category.slug.clone(), index))
            .collect();

        products.sort_by_key(|product| product.id);
        for (index, product) in products.iter_mut().enumerate() {
            if product.id as usize != index + 1 {
                return Err(format!("product ids must be 1..={}", index + 1).into());
            }
            let category = *category_by_slug
                .get(&product.category)
                .ok_or_else(|| format!("product {} has unknown category", product.id))?;
            product.category_index = category;
            categories[category].product_count += 1;
        }
        let product_count = products.len();
        for product in &products {
            if product
                .related
                .iter()
                .any(|&id| id == 0 || id as usize > product_count)
            {
                return Err(format!("product {} has unknown related ids", product.id).into());
            }
        }

        let sort_indexes = |indexes: Vec<usize>| -> SortedIndexes {
            Sort::ALL.map(|sort| {
                let mut sorted = indexes.clone();
                sorted.sort_by(|&a, &b| sort.compare(&products[a], &products[b]));
                sorted
            })
        };
        let sorted_all = sort_indexes((0..products.len()).collect());
        let sorted_by_category = (0..categories.len())
            .map(|category| {
                sort_indexes(
                    (0..products.len())
                        .filter(|&index| products[index].category_index == category)
                        .collect(),
                )
            })
            .collect();
        let featured = (0..products.len())
            .filter(|&index| products[index].featured)
            .collect();

        orders.sort_by(|a, b| b.placed_at.cmp(&a.placed_at).then(b.id.cmp(&a.id)));
        let order_summary = OrderSummary::new(&orders);

        Ok(Self {
            categories: categories.into(),
            category_by_slug,
            products: products.into_iter().map(Arc::new).collect(),
            sorted_all,
            sorted_by_category,
            featured,
            orders: orders.into(),
            order_summary,
        })
    }

    /// All categories, in data order.
    pub fn categories(&self) -> &Arc<[Category]> {
        &self.categories
    }

    pub fn category(&self, index: usize) -> &Category {
        &self.categories[index]
    }

    pub fn category_index(&self, slug: &str) -> Option<usize> {
        self.category_by_slug.get(slug).copied()
    }

    pub fn product_count(&self) -> usize {
        self.products.len()
    }

    /// Looks up a product by id.
    pub fn product(&self, id: u32) -> Option<&Arc<Product>> {
        self.products.get((id as usize).checked_sub(1)?)
    }

    pub fn product_at(&self, index: usize) -> &Product {
        &self.products[index]
    }

    /// The category a product belongs to.
    pub fn category_of(&self, product: &Product) -> &Category {
        &self.categories[product.category_index]
    }

    /// The summary of a product shown on product cards.
    pub fn summary(&self, product: &Product) -> ProductSummary {
        ProductSummary {
            id: product.id,
            name: product.name.clone(),
            category_name: self.category_of(product).name.clone(),
            tagline: product.tagline.clone(),
            price_cents: product.price_cents,
            compare_at_cents: product.compare_at_cents,
            rating: product.rating,
            review_count: product.review_count,
            stock: product.stock,
            color: product.color,
        }
    }

    /// The related products of a product, in data order.
    pub fn related(&self, product: &Product) -> impl Iterator<Item = &Product> {
        product
            .related
            .iter()
            .filter_map(|&id| self.product(id).map(Arc::as_ref))
    }

    /// The featured products, by ascending id.
    pub fn featured(&self) -> impl Iterator<Item = &Product> {
        self.featured.iter().map(|&index| self.product_at(index))
    }

    /// Product indexes of a category, or of all products, in the given order.
    pub fn sorted(&self, category: Option<usize>, sort: Sort) -> &[usize] {
        let indexes = match category {
            Some(category) => &self.sorted_by_category[category],
            None => &self.sorted_all,
        };
        &indexes[sort as usize]
    }

    /// All orders, newest first.
    pub fn orders(&self) -> &Arc<[Order]> {
        &self.orders
    }

    pub fn order_summary(&self) -> &OrderSummary {
        &self.order_summary
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, Box<dyn Error>> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()).into())
}
