use serde::{Deserialize, Serialize};

/// Number of products on one listing page.
pub const PAGE_SIZE: usize = 24;

/// The raw query parameters of a product listing. Empty values count as absent.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct ListingParams {
    pub category: Option<String>,
    pub sort: Option<String>,
    pub page: Option<String>,
}
