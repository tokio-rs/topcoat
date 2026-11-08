pub(crate) mod product_id;

use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::{href, page},
    view::{View, view},
};

use self::product_id::PRODUCTS;

#[page(with(Metadata::new().title("Products")))]
pub(crate) async fn products(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <h1>"Products"</h1>
        <ul>
            for product in PRODUCTS {
                <li>
                    <a href=(href!(product_id::page, product_id: product.id))>
                        (product.name)
                    </a>
                </li>
            }
        </ul>
    })
}
