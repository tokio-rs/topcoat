use std::cmp::Ordering;

use crate::model::{Category, ListingParams, PAGE_SIZE, PageError, Product, Sort};

use super::Store;

/// One page of a product listing, resolved against the store.
pub struct Listing<'a> {
    pub category: Option<&'a Category>,
    pub sort: Sort,
    pub page: usize,
    pub page_count: usize,
    pub total: usize,
    store: &'a Store,
    indexes: &'a [usize],
}

impl<'a> Listing<'a> {
    /// Resolves listing parameters against the store.
    ///
    /// Parameters are validated in order: category, then sort, then page.
    pub fn new(store: &'a Store, params: &ListingParams) -> Result<Self, ListingError> {
        let category = match non_empty(&params.category) {
            None => None,
            Some(slug) => Some(
                store
                    .category_index(slug)
                    .ok_or(ListingError::CategoryNotFound)?,
            ),
        };
        let sort = match non_empty(&params.sort) {
            None => Sort::default(),
            Some(value) => Sort::from_param(value).ok_or(ListingError::InvalidSort)?,
        };
        let sorted = store.sorted(category, sort);
        let total = sorted.len();
        let page_count = total.div_ceil(PAGE_SIZE).max(1);
        let page = match non_empty(&params.page) {
            None => 1,
            Some(value) => parse_page(value)?,
        };
        if page > page_count {
            return Err(ListingError::PageNotFound);
        }
        let start = (page - 1) * PAGE_SIZE;
        let end = (start + PAGE_SIZE).min(total);
        Ok(Self {
            category: category.map(|index| store.category(index)),
            sort,
            page,
            page_count,
            total,
            store,
            indexes: &sorted[start..end],
        })
    }

    /// The products on this page.
    pub fn products(&self) -> impl ExactSizeIterator<Item = &'a Product> + use<'a> {
        let store = self.store;
        self.indexes
            .iter()
            .map(move |&index| store.product_at(index))
    }
}

/// Why listing parameters could not be resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListingError {
    CategoryNotFound,
    InvalidSort,
    InvalidPage,
    PageNotFound,
}

impl From<ListingError> for PageError {
    fn from(error: ListingError) -> Self {
        match error {
            ListingError::CategoryNotFound | ListingError::PageNotFound => Self::NotFound,
            ListingError::InvalidSort | ListingError::InvalidPage => Self::Invalid,
        }
    }
}

impl Sort {
    /// Orders two products. Ties are broken by ascending id.
    pub(super) fn compare(self, a: &Product, b: &Product) -> Ordering {
        let primary = match self {
            Self::Newest => b.created_at.cmp(&a.created_at),
            Self::PriceAsc => a.price_cents.cmp(&b.price_cents),
            Self::PriceDesc => b.price_cents.cmp(&a.price_cents),
            Self::Rating => b
                .rating_tenths()
                .cmp(&a.rating_tenths())
                .then(b.review_count.cmp(&a.review_count)),
        };
        primary.then(a.id.cmp(&b.id))
    }
}

fn non_empty(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|value| !value.is_empty())
}

/// Parses a page number made of ASCII digits. Numbers too large to represent are past any last
/// page.
fn parse_page(value: &str) -> Result<usize, ListingError> {
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ListingError::InvalidPage);
    }
    match value.parse::<usize>() {
        Ok(0) => Err(ListingError::InvalidPage),
        Ok(page) => Ok(page),
        Err(_) => Err(ListingError::PageNotFound),
    }
}
