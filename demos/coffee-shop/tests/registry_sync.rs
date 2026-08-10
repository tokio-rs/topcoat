//! Guards that the theme and components vendored into this demo stay in sync
//! with the built-in registry, which lives in the same workspace. When a
//! registry source changes, these tests fail until the demo is refreshed with
//! the `topcoat ui` commands named in the failure message.
//!
//! The demo vendors only the components it uses, so a component the registry
//! offers but the demo never installed is not a failure. Every vendored file
//! must still come from the registry, which is checked separately.

use std::path::{Path, PathBuf};

use topcoat_ui::Registry;

/// This demo package's root, where `topcoat ui` installed the theme and
/// components.
fn package_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// The directory `topcoat ui` installs components into.
fn components_dir() -> PathBuf {
    package_root().join("src/components")
}

/// The built-in registry, loaded from its crate in this workspace.
fn registry() -> Registry {
    let dir = package_root().join("../../crates/topcoat-ui/registry");
    Registry::load(dir).expect("the workspace's built-in registry loads")
}

/// Reads an installed file, failing with `hint` when it is missing.
fn read_installed(path: &PathBuf, hint: &str) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}; {hint}", path.display()))
}

#[test]
fn theme_matches_registry() {
    let registry = registry();
    let theme = registry
        .theme("neutral")
        .expect("the registry offers the neutral theme this demo installed");

    let installed = package_root().join(theme.file_name());
    let hint = "re-install the theme by deleting it along with components.toml \
        and running `topcoat ui init --theme neutral` in demos/coffee-shop";
    assert!(
        read_installed(&installed, hint) == theme.read_source().unwrap(),
        "{} no longer matches the registry's neutral theme; {hint}",
        installed.display(),
    );
}

#[test]
fn components_match_registry() {
    let registry = registry();

    for name in registry.names() {
        let component = registry.get(name).expect("name came from the registry");
        let installed = components_dir().join(component.file_name());
        if !installed.exists() {
            continue;
        }
        let hint = format!("run `topcoat ui add {name} --overwrite` in demos/coffee-shop");
        assert!(
            read_installed(&installed, &hint) == component.read_source().unwrap(),
            "{} no longer matches the registry's `{name}` component; {hint}",
            installed.display(),
        );
    }
}

#[test]
fn vendored_components_come_from_the_registry() {
    let registry = registry();
    let known: Vec<String> = registry
        .names()
        .map(|name| {
            registry
                .get(name)
                .expect("name came from the registry")
                .file_name()
                .to_owned()
        })
        .collect();

    let dir = components_dir();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", dir.display()));
    for entry in entries {
        let file = entry
            .unwrap_or_else(|error| panic!("cannot read an entry of {}: {error}", dir.display()))
            .file_name();
        let file = file.to_str().expect("component file names are UTF-8");
        assert!(
            known.iter().any(|known| known == file),
            "{} is not a component of the built-in registry; \
             remove it with `topcoat ui remove` in demos/coffee-shop, or move it out of {}",
            dir.join(file).display(),
            dir.display(),
        );
    }
}
