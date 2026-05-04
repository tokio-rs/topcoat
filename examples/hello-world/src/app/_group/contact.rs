use topcoat::{
    router::{Result, page},
    view::view,
};

#[page]
async fn contact_page() -> Result {
    view! { "contact" }
}
