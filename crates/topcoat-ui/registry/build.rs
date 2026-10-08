fn main() {
    // Stage the Lucide icon set for the `iconify_icon!` references in the
    // `#[cfg(test)]` component sources. Gated behind the `stage-icons` feature
    // so ordinary builds and docs.rs stay offline; see the feature in
    // `Cargo.toml`.
    #[cfg(feature = "stage-icons")]
    topcoat_icon::iconify::BuildConfig::new()
        .icon_set("lucide")
        .stage()
        .unwrap();

    #[cfg(feature = "embedded")]
    embed_files();
}

/// Writes `embedded.rs` to `OUT_DIR`: a `(path, contents)` slice holding the
/// manifest and every theme and component source it names. The contents are
/// read with `include_str!`, so editing a source recompiles the crate.
#[cfg(feature = "embedded")]
fn embed_files() {
    use std::{fmt::Write as _, path::PathBuf};

    const MANIFEST: &str = "registry.toml";
    println!("cargo::rerun-if-changed={MANIFEST}");

    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let manifest: toml::Table = std::fs::read_to_string(root.join(MANIFEST))
        .unwrap()
        .parse()
        .unwrap();

    let mut paths = vec![MANIFEST];
    for section in ["themes", "components"] {
        let Some(entries) = manifest.get(section).and_then(toml::Value::as_table) else {
            continue;
        };
        for (name, entry) in entries {
            let source = entry
                .get("source")
                .and_then(toml::Value::as_str)
                .unwrap_or_else(|| panic!("`{section}.{name}` has no `source` path"));
            paths.push(source);
        }
    }

    let mut out = String::from("&[\n");
    for path in paths {
        writeln!(
            out,
            "    ({path:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/\", {path:?}))),"
        )
        .unwrap();
    }
    out.push(']');

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(out_dir.join("embedded.rs"), out).unwrap();
}
