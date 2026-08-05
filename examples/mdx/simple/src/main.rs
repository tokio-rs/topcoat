use topcoat::{
    Result,
    mdx::compile_mdx,
    router::{Router, page},
};

// --- Server -----------------------------------------------------------------

#[tokio::main]
async fn main() {
    topcoat::start(router()).await.unwrap();
}

// --- Router -----------------------------------------------------------------

fn router() -> Router {
    Router::builder().page(home).page(about).build()
}

// --- Pages ------------------------------------------------------------------

#[page("/")]
async fn home() -> Result {
    compile_mdx!("pages/home.mdx")
}

#[page("/about")]
async fn about() -> Result {
    compile_mdx!("pages/about.mdx")
}
