//! The routes of the store. Every internal link is built from a [`Route`] value.

use std::fmt::{self, Display, Formatter};

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{model::Sort, pages::*, views::Shell};

/// The pages of the store. Every page renders inside [`Shell`].
#[derive(Clone, Debug, PartialEq, Routable)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Shell)]
        #[route("/")]
        Home {},
        #[route("/about")]
        About {},
        #[route("/products?:..query")]
        Listing { query: ListingQuery },
        #[route("/products/:id")]
        Detail { id: u32 },
        #[route("/admin/orders")]
        Orders {},
        #[route("/:..segments")]
        NotFound { segments: Vec<String> },
}

impl Route {
    /// The first page of a category in the default order.
    pub fn category(slug: &str) -> Self {
        Self::listing(Some(slug), Sort::default(), 1)
    }

    /// A page of the listing. Parameters at their default are left out of the URL.
    pub fn listing(category: Option<&str>, sort: Sort, page: usize) -> Self {
        Self::Listing {
            query: ListingQuery {
                category: category.map(str::to_owned),
                sort: (sort != Sort::default()).then(|| sort.param().to_owned()),
                page: (page != 1).then(|| page.to_string()),
            },
        }
    }
}

/// The `category`, `sort` and `page` query parameters of the listing, as they appear in the URL.
///
/// The values stay unvalidated strings so that the server can answer invalid ones with the 404 and
/// 422 pages. Empty values count as absent.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct ListingQuery {
    pub category: Option<String>,
    pub sort: Option<String>,
    pub page: Option<String>,
}

/// Parses the query string, which the router has already percent-decoded.
impl From<&str> for ListingQuery {
    fn from(query: &str) -> Self {
        let mut parsed = Self::default();
        for pair in query.split('&') {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            let slot = match name {
                "category" => &mut parsed.category,
                "sort" => &mut parsed.sort,
                "page" => &mut parsed.page,
                _ => continue,
            };
            *slot = Some(value.to_owned());
        }
        parsed
    }
}

/// Writes the parameters in the order `category`, `sort`, `page`, leaving out empty ones and the
/// defaults `sort=newest` and `page=1`, so that equivalent queries give the same URL.
impl Display for ListingQuery {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let not = |default: &'static str| move |value: &&str| *value != default;
        let params = [
            ("category", self.category.as_deref()),
            ("sort", self.sort.as_deref().filter(not(Sort::default().param()))),
            ("page", self.page.as_deref().filter(not("1"))),
        ];
        let mut separator = "";
        for (name, value) in params {
            if let Some(value) = value.filter(|value| !value.is_empty()) {
                write!(f, "{separator}{name}={value}")?;
                separator = "&";
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_urls_leave_out_defaults() {
        let url = |category, sort, page| Route::listing(category, sort, page).to_string();
        assert_eq!(
            url(Some("outdoor"), Sort::PriceAsc, 2),
            "/products?category=outdoor&sort=price-asc&page=2"
        );
        assert_eq!(url(None, Sort::Newest, 3), "/products?page=3");
        assert_eq!(url(Some("outdoor"), Sort::Newest, 1), "/products?category=outdoor");
    }

    #[test]
    fn equivalent_queries_display_the_same() {
        let route: Route = "/products?category=outdoor&sort=newest&page=1".parse().unwrap();
        assert_eq!(route.to_string(), Route::category("outdoor").to_string());
    }

    #[test]
    fn listing_urls_round_trip() {
        let route = Route::listing(Some("kitchen"), Sort::Rating, 2);
        assert_eq!(route.to_string().parse::<Route>().unwrap(), route);
    }

    #[test]
    fn invalid_values_reach_the_page() {
        let route: Route = "/products?sort=bogus&page=abc".parse().unwrap();
        let Route::Listing { query } = route else {
            panic!("expected the listing route");
        };
        assert_eq!(query.sort.as_deref(), Some("bogus"));
        assert_eq!(query.page.as_deref(), Some("abc"));
    }
}
