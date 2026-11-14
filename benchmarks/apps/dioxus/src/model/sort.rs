use serde::{Deserialize, Serialize};

/// The order of products in a listing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum Sort {
    #[default]
    Newest,
    PriceAsc,
    PriceDesc,
    Rating,
}

impl Sort {
    pub const ALL: [Self; 4] = [Self::Newest, Self::PriceAsc, Self::PriceDesc, Self::Rating];

    #[cfg(feature = "server")]
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
}
