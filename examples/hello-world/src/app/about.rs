use topcoat::{
    router::page,
    view::{View, view},
};

#[page]
async fn about_page() -> View {
    view! { "about" }
}
