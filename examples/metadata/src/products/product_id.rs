use topcoat::{
    Result,
    context::{Cx, memoize},
    metadata::Metadata,
    router::{error::RouterErrorExt, module_param, page, path_param},
    view::{View, view},
};

module_param!(pub(crate) product_id: u32, error = not_found);

#[derive(Clone)]
pub(crate) struct Product {
    pub(crate) id: u32,
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
}

pub(crate) const PRODUCTS: &[Product] = &[
    Product {
        id: 1,
        name: "Teapot",
        description: "A sturdy teapot that brews six cups.",
    },
    Product {
        id: 2,
        name: "Kettle",
        description: "An electric kettle with a temperature dial.",
    },
];

/// Finds the product of the current request, or responds 404.
///
/// The metadata and the page both call it. `#[memoize]` makes them share one
/// lookup, which matters once it queries a database.
#[memoize(as_ref)]
async fn find_product(cx: &Cx) -> Result<Product> {
    let id = *path_param::<ProductId>(cx)?;
    Ok(PRODUCTS
        .iter()
        .find(|product| product.id == id)
        .cloned()
        .ok_or_not_found()?)
}

// The title and description depend on the product, so they are loaded for
// each request. The head waits for them before the page starts streaming.
#[page(with(Metadata::from_fn(async |cx| {
    let product = find_product(cx).await?;
    Ok(Metadata::new()
        .title(product.name)
        .description(product.description))
})))]
pub(crate) async fn page(cx: &Cx) -> Result<impl View> {
    let product = find_product(cx).await?;

    Ok(view! {
        <h1>(product.name)</h1>
        <p>(product.description)</p>
    })
}
