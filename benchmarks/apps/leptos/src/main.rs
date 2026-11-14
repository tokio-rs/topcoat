//! The Harbor Market server: Leptos pages and plain Axum JSON routes on one Axum router.

mod config;

use std::{net::Ipv4Addr, process::ExitCode, sync::Arc};

use axum::{
    Router,
    extract::FromRef,
    routing::{get, post},
};
use harbor_leptos::{
    api,
    app::{App, shell},
    store::Store,
};
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, file_and_error_handler_with_context, generate_route_list};
use tokio::{net::TcpListener, runtime};
use tower_http::compression::CompressionLayer;

use crate::config::Config;

#[cfg(all(feature = "jemalloc", feature = "mimalloc"))]
compile_error!("the `jemalloc` and `mimalloc` features are mutually exclusive");

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// State shared by the Axum handlers.
#[derive(Clone, FromRef)]
struct AppState {
    leptos_options: LeptosOptions,
    store: Arc<Store>,
}

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
    let leptos_options = match get_configuration(None) {
        Ok(configuration) => configuration.leptos_options,
        Err(error) => {
            eprintln!("error: invalid Leptos configuration: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut builder = runtime::Builder::new_multi_thread();
    builder.enable_all();
    if let Some(threads) = config.threads {
        builder.worker_threads(threads);
    }
    let runtime = builder.build().expect("failed to build the Tokio runtime");

    let result = runtime.block_on(async {
        let state = AppState {
            leptos_options,
            store,
        };
        let app = router(state, config.compression);
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

fn router(state: AppState, compression: bool) -> Router {
    // Pages and server functions find the store in the Leptos context.
    let provide_store = {
        let store = state.store.clone();
        move || provide_context(store.clone())
    };
    let options = state.leptos_options.clone();
    let router = Router::new()
        .route("/health", get(health))
        .route("/api/products", get(api::list_products))
        .route("/api/products/{id}", get(api::product))
        .route("/api/cart", post(api::add_to_cart))
        .leptos_routes_with_context(
            &state,
            generate_route_list(App),
            provide_store.clone(),
            move || shell(options.clone()),
        )
        .fallback(file_and_error_handler_with_context::<AppState, _>(
            provide_store,
            shell,
        ))
        .with_state(state);
    if compression {
        router.layer(CompressionLayer::new())
    } else {
        router
    }
}

async fn health() -> &'static str {
    "ok"
}
