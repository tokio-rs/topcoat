use leptos::prelude::*;

use crate::model::{HomeData, ProductDetailData, ProductsData};

#[server]
pub async fn get_home() -> Result<HomeData, ServerFnError> {
    use crate::catalog::{Product, CATALOG};

    Ok(HomeData {
        featured: CATALOG.featured().map(Product::summary).collect(),
    })
}

#[server]
pub async fn get_products(
    page: usize,
    sort: Option<String>,
    category: Option<String>,
) -> Result<ProductsData, ServerFnError> {
    use crate::{catalog::CATALOG, format::normalize_sort};

    let sort = normalize_sort(sort.as_deref()).map(str::to_owned);
    let page_data = CATALOG.page(page, sort.as_deref(), category.as_deref());

    Ok(ProductsData {
        categories: CATALOG.categories(),
        items: page_data
            .items
            .iter()
            .map(|product| product.summary())
            .collect(),
        current: page_data.current,
        page_count: page_data.page_count,
        total: page_data.total,
        sort,
        category,
    })
}

#[server]
pub async fn get_product(id: u32) -> Result<Option<ProductDetailData>, ServerFnError> {
    use crate::catalog::CATALOG;

    Ok(CATALOG.get(id).map(|product| product.detail(&CATALOG)))
}
