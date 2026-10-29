pub(crate) mod product_id;

use topcoat::{
    Result,
    metadata::Metadata,
    router::{href, page},
    view::{View, view},
};

use self::product_id::{PRODUCTS, ProductId};

#[page(with(Metadata::new().title("Products")))]
pub(crate) async fn products() -> Result<impl View> {
    Ok(view! {
        <h1>"Products"</h1>
        <ul>
            for product in PRODUCTS {
                <li>
                    <a href=(href!(product_id::page, ProductId(product.id)))>
                        (product.name)
                    </a>
                </li>
            }
        </ul>
    })
}
