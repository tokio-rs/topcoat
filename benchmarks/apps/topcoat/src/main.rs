//! Harbor Market storefront built with Topcoat.

mod app;
mod cart;
mod components;
mod config;
mod data;
mod format;

use std::process::ExitCode;

use tokio::runtime;

use crate::{config::Config, data::Store};

// `jemalloc` takes precedence so that `--all-features` builds still compile.
#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(all(feature = "mimalloc", not(feature = "jemalloc")))]
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
        Ok(store) => store,
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

    // `start` binds the `HOST` and `PORT` environment variables, defaulting to 127.0.0.1.
    let router = app::router(store, config.compression);
    match runtime.block_on(topcoat::start(router)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
