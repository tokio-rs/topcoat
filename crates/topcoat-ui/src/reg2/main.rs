//! Generates `registry.toml` for the example `reg2` registry.
//!
//! This registry exists only to exercise cross-registry dependencies.
//! Run with `cargo run -p topcoat-ui --bin reg2-registry --features generate`.

use std::path::Path;
use std::process::ExitCode;

use topcoat_ui::generate::{Component, Registry};

fn main() -> ExitCode {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/reg2");
    let registry = Registry::new("reg2").component(
        Component::new("gadget", "gadget.rs").dep_from("file://../reg3", "foo"),
    );

    match registry.generate(&dir) {
        Ok(path) => {
            println!("wrote {}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
