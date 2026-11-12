use dioxus::prelude::StatusCode;

use super::Store;
use crate::model::{Category, Product, Sort};

/// Number of products on one listing page.
pub const PAGE_SIZE: usize = 24;

/// One page of a product listing.
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
    /// Resolves the raw `category`, `sort` and `page` query parameters against the store. Empty
    /// values count as absent.
    ///
    /// Parameters are validated in order: category, then sort, then page.
    pub fn new(
        store: &'a Store,
        category: Option<&str>,
        sort: Option<&str>,
        page: Option<&str>,
    ) -> Result<Self, ListingError> {
        let category = match non_empty(category) {
            None => None,
            Some(slug) => Some(
                store
                    .category_index(slug)
                    .ok_or(ListingError::CategoryNotFound)?,
            ),
        };
        let sort = match non_empty(sort) {
            None => Sort::default(),
            Some(value) => Sort::from_param(value).ok_or(ListingError::InvalidSort)?,
        };
        let sorted = store.sorted(category, sort);
        let total = sorted.len();
        let page_count = total.div_ceil(PAGE_SIZE).max(1);
        let page = match non_empty(page) {
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

    /// 1-based position of the first product on this page, or 0 if the listing is empty.
    pub fn first_position(&self) -> usize {
        if self.total == 0 {
            0
        } else {
            (self.page - 1) * PAGE_SIZE + 1
        }
    }

    /// 1-based position of the last product on this page.
    pub fn last_position(&self) -> usize {
        (self.page - 1) * PAGE_SIZE + self.indexes.len()
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

impl ListingError {
    pub fn status(self) -> StatusCode {
        match self {
            Self::CategoryNotFound | Self::PageNotFound => StatusCode::NOT_FOUND,
            Self::InvalidSort | Self::InvalidPage => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::CategoryNotFound => "category not found",
            Self::InvalidSort => "invalid sort",
            Self::InvalidPage => "invalid page",
            Self::PageNotFound => "page not found",
        }
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.is_empty())
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
