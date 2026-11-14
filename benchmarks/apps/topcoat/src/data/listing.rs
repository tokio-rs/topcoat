use serde::Serialize;
use topcoat::router::query_params;

use super::{Category, Product, Store};

/// Number of products on one listing page.
pub const PAGE_SIZE: usize = 24;

/// The raw query parameters of a product listing. Empty values count as absent.
#[query_params]
pub struct ListingParams {
    pub category: Option<String>,
    pub sort: Option<String>,
    pub page: Option<String>,
}

/// The order of products in a listing. Serializes as its `sort` query parameter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sort {
    #[default]
    Newest,
    PriceAsc,
    PriceDesc,
    Rating,
}

impl Sort {
    pub const ALL: [Self; 4] = [Self::Newest, Self::PriceAsc, Self::PriceDesc, Self::Rating];

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde passes fields by reference"
    )]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn from_param(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|sort| sort.param() == value)
    }

    /// The value of the `sort` query parameter.
    pub fn param(self) -> &'static str {
        match self {
            Self::Newest => "newest",
            Self::PriceAsc => "price-asc",
            Self::PriceDesc => "price-desc",
            Self::Rating => "rating",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Newest => "Newest",
            Self::PriceAsc => "Price: low to high",
            Self::PriceDesc => "Price: high to low",
            Self::Rating => "Top rated",
        }
    }

    pub(super) fn index(self) -> usize {
        self as usize
    }

    /// Orders two products. Ties are broken by ascending id.
    pub(super) fn compare(self, a: &Product, b: &Product) -> std::cmp::Ordering {
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
    /// Resolves listing parameters against the store.
    ///
    /// Parameters are validated in order: category, then sort, then page.
    pub fn new(store: &'a Store, params: &ListingParams) -> Result<Self, ListingError> {
        let category = match params.category.as_deref() {
            None => None,
            Some(slug) => Some(
                store
                    .category_index(slug)
                    .ok_or(ListingError::CategoryNotFound)?,
            ),
        };
        let sort = match params.sort.as_deref() {
            None => Sort::default(),
            Some(value) => Sort::from_param(value).ok_or(ListingError::InvalidSort)?,
        };
        let sorted = store.sorted(category, sort);
        let total = sorted.len();
        let page_count = total.div_ceil(PAGE_SIZE).max(1);
        let page = match params.page.as_deref() {
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

    pub fn category_slug(&self) -> Option<&'a str> {
        self.category.map(|category| category.slug.as_str())
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
