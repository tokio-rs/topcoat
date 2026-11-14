//! Harbor Market storefront built with Axum, Maud and tower-http.

mod api;
mod assets;
mod cart;
mod config;
mod data;
mod format;
mod pages;
mod urls;
mod views;

use std::{net::Ipv4Addr, process::ExitCode, sync::Arc};

use axum::{
    Router,
    routing::{get, post},
};
use tokio::{net::TcpListener, runtime};
use tower_http::compression::CompressionLayer;

use crate::{config::Config, data::Store};

#[cfg(all(feature = "jemalloc", feature = "mimalloc"))]
compile_error!("the `jemalloc` and `mimalloc` features are mutually exclusive");

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> ExitCode {
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let store = match Store::load(&config.data_dir) {
        Ok(store) => Arc::new(store),
        Err(error) => {
            eprintln!("error: failed to load data: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut builder = runtime::Builder::new_multi_thread();
    builder.enable_all();
    if let Some(threads) = config.threads {
        builder.worker_threads(threads);
    }
    let runtime = builder.build().expect("failed to build the Tokio runtime");

    let app = router(store, config.compression);
    let result = runtime.block_on(async {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, config.port)).await?;
        axum::serve(listener, app).await
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn router(store: Arc<Store>, compression: bool) -> Router {
    let router = Router::new()
        .route("/health", get(health))
        .route("/", get(pages::home_page))
        .route("/about", get(pages::about_page))
        .route("/products", get(pages::listing_page))
        .route("/products/{id}", get(pages::detail_page))
        .route("/admin/orders", get(pages::orders_page))
        .route("/api/products", get(api::list_products))
        .route("/api/products/{id}", get(api::product))
        .route("/api/cart", post(api::add_to_cart))
        .merge(assets::router())
        .fallback(pages::not_found_page)
        .with_state(store);
    if compression {
        router.layer(CompressionLayer::new())
    } else {
        router
    }
}

async fn health() -> &'static str {
    "ok"
}
