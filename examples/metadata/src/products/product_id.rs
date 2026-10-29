use topcoat::{
    Result,
    context::{Cx, memoize},
    metadata::{Image, Metadata},
    router::{error::RouterErrorExt, module_param, page, path_param},
    view::{View, view},
};

module_param!(pub(crate) product_id: u32 else not_found);

#[derive(Clone)]
pub(crate) struct Product {
    pub(crate) id: u32,
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) image: &'static str,
}

pub(crate) const PRODUCTS: &[Product] = &[
    Product {
        id: 1,
        name: "Teapot",
        description: "A sturdy teapot that brews six cups.",
        image: "/images/teapot.png",
    },
    Product {
        id: 2,
        name: "Kettle",
        description: "An electric kettle with a temperature dial.",
        image: "/images/kettle.png",
    },
];

/// Looks up the requested product, returning a 404 error if it does not exist.
///
/// Both the metadata loader and the page call this function. `#[memoize]`
/// shares the result so a database query added here would run only once.
#[memoize(as_ref)]
async fn find_product(cx: &Cx) -> Result<Product> {
    let id = *path_param::<ProductId>(cx)?;
    Ok(PRODUCTS
        .iter()
        .find(|product| product.id == id)
        .cloned()
        .ok_or_not_found()?)
}

// Load the selected product's title, description, and preview image for each
// request. The metadata tags wait for these values before they render.
#[page(with(Metadata::from_fn(async |cx| {
    let product = find_product(cx).await?;
    Ok(Metadata::new()
        .title(product.name)
        .description(product.description)
        .open_graph(|og| {
            og.image(
                Image::new(product.image)
                    .size(1200, 630)
                    .alt(product.name),
            )
        }))
})))]
pub(crate) async fn page(cx: &Cx) -> Result<impl View> {
    let product = find_product(cx).await?;

    Ok(view! {
        <h1>(product.name)</h1>
        <p>(product.description)</p>
    })
}
