use topcoat::{
    router::page,
    view::{View, view},
};

#[page]
async fn contact_page() -> View {
    view! { "contact" }
}
