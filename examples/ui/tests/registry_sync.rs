//! Guards that the theme and components vendored into this example stay in
//! sync with the built-in registry, which lives in the same workspace. When a
//! registry source changes (or the registry gains a component this example
//! does not showcase yet), these tests fail until the example is refreshed
//! with the `topcoat ui` commands named in the failure message.

use std::path::{Path, PathBuf};

use topcoat_ui::Registry;

/// This example package's root, where `topcoat ui` installed the theme and
/// components.
fn package_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
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
        .expect("the registry offers the neutral theme this example installed");

    let installed = package_root().join(theme.file_name());
    let hint = "re-install the theme by deleting it along with components.toml \
        and running `topcoat ui init --theme neutral` in examples/ui";
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
        let installed = package_root()
            .join("src/components")
            .join(component.file_name());
        let hint = format!(
            "run `topcoat ui add {name} --overwrite` in examples/ui \
             (and showcase the component if it is new)"
        );
        assert!(
            read_installed(&installed, &hint) == component.read_source().unwrap(),
            "{} no longer matches the registry's `{name}` component; {hint}",
            installed.display(),
        );
    }
}
