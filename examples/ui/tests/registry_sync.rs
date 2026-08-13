//! Guards that the theme and components vendored into this example stay in
//! sync with the built-in registry, which lives in the same workspace. Two
//! things can drift: the installed files, which are a verbatim copy of the
//! registry source, and the hashes `components.toml` records for them, which
//! `topcoat ui list` compares against the registry to report component
//! updates. A stale hash offers an update for a file that already carries it.
//! When a registry source changes (or the registry gains a component this
//! example does not showcase yet), these tests fail until the example is
//! refreshed against the registry.

use std::path::{Path, PathBuf};

use topcoat_ui::{
    DEFAULT_REGISTRY, Registry,
    manage::{InstallStatus, Package, list},
};

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

/// Reads an installed file, failing with `hint`, where there is one saying how
/// to put the file back.
fn read_installed(path: &PathBuf, hint: Option<&str>) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| match hint {
        Some(hint) => panic!("cannot read {}: {error}; {hint}", path.display()),
        None => panic!("cannot read {}: {error}", path.display()),
    })
}

/// What to run when the component `name` has drifted from the registry.
fn component_hint(name: &str) -> String {
    format!("run `topcoat ui add {name} --overwrite` in examples/ui")
}

#[test]
fn theme_matches_registry() {
    let registry = registry();
    let theme = registry
        .theme("neutral")
        .expect("the registry offers the neutral theme this example installed");

    let installed = package_root().join(theme.file_name());
    assert!(
        read_installed(&installed, None) == theme.read_source().unwrap(),
        "{} no longer matches the registry's neutral theme",
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
        let hint = component_hint(name);
        assert!(
            read_installed(&installed, Some(&hint)) == component.read_source().unwrap(),
            "{} no longer matches the registry's `{name}` component; {hint}",
            installed.display(),
        );
    }
}

#[test]
fn components_are_up_to_date() {
    let package = Package::locate(Some(String::from("ui"))).expect("the ui example is a package");
    let listings =
        list(&package, Some(DEFAULT_REGISTRY)).expect("the built-in registry can be listed");
    let statuses = listings
        .into_iter()
        .next()
        .expect("listing the built-in registry yields it")
        .outcome
        .expect("the built-in registry loads");

    for status in statuses {
        let name = &status.name;
        match status.status {
            InstallStatus::UpToDate { .. } => {}
            InstallStatus::Update { .. } => panic!(
                "`{name}` has an update available; {hint}",
                hint = component_hint(name),
            ),
            InstallStatus::Available { .. } => panic!(
                "the registry offers `{name}`, which this example has not installed; \
                 run `topcoat ui add {name}` in examples/ui and showcase it",
            ),
            InstallStatus::Orphaned { .. } => panic!(
                "this example has `{name}` installed, which the registry no longer offers; \
                 run `topcoat ui remove {name}` in examples/ui",
            ),
        }
    }
}
