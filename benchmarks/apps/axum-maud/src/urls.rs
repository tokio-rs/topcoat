//! URL builders for links between pages.

use std::fmt::{self, Display, Formatter};

use crate::data::Sort;

/// The URL of a product detail page.
pub struct ProductUrl(pub u32);

impl Display for ProductUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "/products/{}", self.0)
    }
}

/// The URL of a listing page. Parameters with default values are left out.
pub struct ListingUrl<'a> {
    pub category: Option<&'a str>,
    pub sort: Sort,
    pub page: usize,
}

impl<'a> ListingUrl<'a> {
    /// The first page of a category in the default order.
    pub fn category(slug: &'a str) -> Self {
        Self {
            category: Some(slug),
            sort: Sort::default(),
            page: 1,
        }
    }
}

impl Display for ListingUrl<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("/products")?;
        let mut separator = '?';
        if let Some(category) = self.category {
            write!(f, "{separator}category={category}")?;
            separator = '&';
        }
        if self.sort != Sort::default() {
            write!(f, "{separator}sort={}", self.sort.param())?;
            separator = '&';
        }
        if self.page != 1 {
            write!(f, "{separator}page={}", self.page)?;
        }
        Ok(())
    }
}
