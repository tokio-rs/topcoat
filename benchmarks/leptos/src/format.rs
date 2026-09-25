//! Formatting and URL helpers compiled for both the server and the client.
//! These mirror the other benchmark apps exactly (the parity contract).

/// Formats a price in cents as dollars, using integer arithmetic.
pub fn format_price(cents: u32) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

/// A rating in tenths formatted as `4.3`.
pub fn format_rating(tenths: u32) -> String {
    format!("{}.{}", tenths / 10, tenths % 10)
}

/// Number of filled stars out of five for a rating in tenths, rounded half-up.
pub fn filled_stars(tenths: u32) -> u32 {
    (tenths + 5) / 10
}

/// First letter of the first two words of the name, used as the image
/// placeholder.
pub fn initials(name: &str) -> String {
    name.split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .collect()
}

/// Builds a `/products` URL with the canonical query parameter order
/// (`page`, `sort`, `category`), omitting defaults.
pub fn products_url(page: usize, sort: Option<&str>, category: Option<&str>) -> String {
    let mut url = String::from("/products");
    let mut sep = '?';

    if page > 1 {
        url.push(sep);
        sep = '&';
        url.push_str("page=");
        url.push_str(&page.to_string());
    }
    if let Some(sort) = sort {
        url.push(sep);
        sep = '&';
        url.push_str("sort=");
        url.push_str(sort);
    }
    if let Some(category) = category {
        url.push(sep);
        url.push_str("category=");
        url.push_str(category);
    }

    url
}

/// Normalizes the requested sort order. Unknown values select ascending ID order.
pub fn normalize_sort(sort: Option<&str>) -> Option<&'static str> {
    match sort {
        Some("name") => Some("name"),
        Some("price") => Some("price"),
        Some("price-desc") => Some("price-desc"),
        Some("rating") => Some("rating"),
        _ => None,
    }
}
