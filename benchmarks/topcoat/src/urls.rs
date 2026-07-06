/// Builds a `/products` URL with the canonical query parameter order
/// (`page`, `sort`, `category`), omitting defaults. Every benchmark app builds
/// identical URLs so rendered documents can be diffed.
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

/// Maps the raw `sort` query value onto the four supported sort orders;
/// anything else falls back to the default (ascending id) order.
pub fn normalize_sort(sort: Option<&str>) -> Option<&'static str> {
    match sort {
        Some("name") => Some("name"),
        Some("price") => Some("price"),
        Some("price-desc") => Some("price-desc"),
        Some("rating") => Some("rating"),
        _ => None,
    }
}
