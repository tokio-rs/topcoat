//! Compiles the Tailwind stylesheet with the standalone `tailwindcss` CLI and derives
//! content-hashed URLs for the stylesheet and the script.
//!
//! The CLI comes from `PATH` and should be [`TAILWIND_VERSION`], the version in the benchmark
//! Nix shell (`nix develop .#benchmarks`), so every app's CSS is built by the same Tailwind.
//! Another version only produces a warning here; the benchmark runner checks tool versions
//! against `expected_tools` in `bench.toml` and records them.

use std::{env, fs, path::PathBuf, process::Command};

const TAILWIND_VERSION: &str = "4.3.3";

fn main() {
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=assets");
    println!("cargo::rerun-if-env-changed=PATH");

    check_tailwind_version();

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let stylesheet = out_dir.join("app.css");
    let status = Command::new("tailwindcss")
        .args(["--input", "assets/app.css", "--output"])
        .arg(&stylesheet)
        .arg("--minify")
        .status()
        .unwrap_or_else(|error| panic!("failed to run `tailwindcss`: {error}"));
    assert!(status.success(), "`tailwindcss` exited with {status}");

    let css = fs::read(&stylesheet).unwrap();
    let js = fs::read("assets/app.js").unwrap();
    let paths = format!(
        "pub const STYLESHEET_PATH: &str = \"/assets/app-{:016x}.css\";\n\
         pub const SCRIPT_PATH: &str = \"/assets/app-{:016x}.js\";\n",
        fnv1a(&css),
        fnv1a(&js),
    );
    fs::write(out_dir.join("asset_paths.rs"), paths).unwrap();
}

/// Warns unless `tailwindcss` on `PATH` reports [`TAILWIND_VERSION`].
fn check_tailwind_version() {
    let output = Command::new("tailwindcss")
        .arg("--help")
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "`tailwindcss` not found on PATH ({error}); build inside `nix develop .#benchmarks`"
            )
        });
    let help = String::from_utf8_lossy(&output.stdout);
    let help = help + String::from_utf8_lossy(&output.stderr);
    let expected = format!("tailwindcss v{TAILWIND_VERSION}");
    let found = help
        .lines()
        .find(|line| line.contains("tailwindcss v"))
        .unwrap_or("unknown");
    let matches = help.lines().any(|line| {
        line.trim_start_matches(|c: char| !c.is_ascii_alphabetic())
            .trim_end()
            == expected
    });
    if !matches {
        println!(
            "cargo::warning=expected {expected} but `tailwindcss` on PATH is {:?}; build inside `nix develop .#benchmarks`",
            found.trim(),
        );
    }
}

/// 64-bit FNV-1a, a small stable hash for cache-busting file names.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}
