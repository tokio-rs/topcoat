//! Generates `registry.toml` for the example `reg3` registry.
//!
//! This registry exists only to exercise cross-registry dependencies and an
//! ambiguous component name (`button`). Run with
//! `cargo run -p topcoat-ui --bin reg3-registry --features generate`.

use std::path::Path;
use std::process::ExitCode;

use topcoat_ui::generate::{Component, Registry};

fn main() -> ExitCode {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/reg3");
    let registry = Registry::new("reg3")
        .component(Component::new("foo", "foo.rs"))
        .component(Component::new("button", "button2.rs"));

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
