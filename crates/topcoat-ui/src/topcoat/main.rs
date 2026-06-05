//! Generates `registry.toml` for the built-in `topcoat` registry.
//!
//! Run with `cargo run -p topcoat-ui --bin topcoat-registry --features generate`.

use std::path::Path;
use std::process::ExitCode;

use topcoat_ui::generate::{Component, Registry};

fn main() -> ExitCode {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/topcoat");
    let registry = Registry::new("topcoat")
        .component(
            Component::new("button", "button.rs")
                .dep("kek")
                .dep_from("file://../reg2", "gadget"),
        )
        .component(Component::new("kek", "kek.rs"));

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
