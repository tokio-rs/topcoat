//! Server startup: configuration, the dataset, the Tokio runtime and the Axum router. Server only.

use std::{env, path::PathBuf, process::ExitCode};

use axum::{
    Router,
    routing::{get, post},
};
use dioxus::logger::tracing::Level;
use tokio::runtime;
use tower_http::compression::CompressionLayer;

use crate::{App, api, store::Store};

/// Server settings read from the benchmark environment variables.
///
/// `PORT` is read by Dioxus itself, which binds `127.0.0.1:$PORT`.
struct Config {
    data_dir: PathBuf,
    /// Number of Tokio worker threads, or `None` for the default of one per core.
    threads: Option<usize>,
    compression: bool,
}

impl Config {
    fn from_env() -> Result<Self, String> {
        let data_dir = match env::var_os("BENCH_DATA_DIR") {
            Some(value) => PathBuf::from(value),
            None => PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data")),
        };
        let threads = match env::var("BENCH_THREADS") {
            Ok(value) => match value.parse() {
                Ok(0) => None,
                Ok(threads) => Some(threads),
                Err(_) => return Err(format!("invalid BENCH_THREADS {value:?}")),
            },
            Err(_) => None,
        };
        let compression = match env::var("BENCH_COMPRESSION").as_deref() {
            Ok("on") => true,
            Ok("off") | Err(_) => false,
            Ok(value) => return Err(format!("invalid BENCH_COMPRESSION {value:?}")),
        };
        Ok(Self {
            data_dir,
            threads,
            compression,
        })
    }
}

pub fn main() -> ExitCode {
    // Warnings and errors only, so nothing is logged per request.
    _ = dioxus::logger::init(Level::WARN);

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = Store::init(&config.data_dir) {
        eprintln!("error: failed to load data: {error}");
        return ExitCode::FAILURE;
    }

    let mut builder = runtime::Builder::new_multi_thread();
    builder.enable_all();
    if let Some(threads) = config.threads {
        builder.worker_threads(threads);
    }
    let runtime = builder.build().expect("failed to build the Tokio runtime");

    // `dioxus::serve` runs on the runtime entered here.
    let _guard = runtime.enter();
    let compression = config.compression;
    dioxus::serve(move || async move {
        let router = router();
        Ok(if compression {
            router.layer(CompressionLayer::new())
        } else {
            router
        })
    })
}

/// The server-rendered Dioxus app with the JSON API and the health check next to it.
///
/// Dioxus renders every path that has no route of its own.
fn router() -> Router {
    dioxus::server::router(App)
        .route("/health", get(api::health))
        .route("/api/products", get(api::list_products))
        .route("/api/products/{id}", get(api::product))
        .route("/api/cart", post(api::add_to_cart))
}
