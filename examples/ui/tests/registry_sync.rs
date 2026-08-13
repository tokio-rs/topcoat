//! Guards that the theme and components vendored into this example stay in
//! sync with the built-in registry, which lives in the same workspace. Two
//! things can drift: the installed files, which are a verbatim copy of the
//! registry source, and the hashes `components.toml` records for them, which
//! `topcoat ui list` compares against the registry to report component
//! updates. A stale hash offers an update for a file that already carries it.
//! When a registry source changes (or the registry gains a component this
//! example does not showcase yet), these tests fail until the example is
//! refreshed against the registry.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use topcoat_ui::{DEFAULT_REGISTRY, Registry};

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
    format!(
        "run `topcoat ui add {name} --overwrite` in examples/ui \
         (and showcase the component if it is new)"
    )
}

/// The parts of this example's `components.toml` these tests read. The file
/// records more than this; the rest is `topcoat ui`'s business.
#[derive(Deserialize)]
struct InstallState {
    theme: InstalledTheme,
    registries: BTreeMap<String, InstalledRegistry>,
}

/// The theme this example installed: which of the registry's themes it came
/// from, and the hash of that theme's source at the time.
#[derive(Deserialize)]
struct InstalledTheme {
    name: String,
    hash: String,
}

/// The components this example installed from one registry, by name.
#[derive(Deserialize)]
struct InstalledRegistry {
    components: BTreeMap<String, InstalledComponent>,
}

/// One installed component: the hash of the registry source it came from.
#[derive(Deserialize)]
struct InstalledComponent {
    hash: String,
}

/// The install state `topcoat ui` wrote at this example's package root.
fn install_state() -> InstallState {
    let path = package_root().join("components.toml");
    let hint = "install the theme and components by running \
        `topcoat ui init --theme neutral` in examples/ui";
    let raw = read_installed(&path, Some(hint));
    toml::from_str(&raw).unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()))
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
fn recorded_hashes_match_registry() {
    let registry = registry();
    let state = install_state();

    let theme = registry
        .theme(&state.theme.name)
        .expect("the registry offers the theme this example installed");
    assert!(
        state.theme.hash == theme.hash().unwrap(),
        "components.toml records a stale hash for the `{}` theme",
        state.theme.name,
    );

    let tracked = &state
        .registries
        .get(DEFAULT_REGISTRY)
        .expect("the example tracks its components under the built-in registry")
        .components;

    for name in registry.names() {
        let component = registry.get(name).expect("name came from the registry");
        let hint = component_hint(name);
        let recorded = tracked
            .get(name)
            .unwrap_or_else(|| panic!("components.toml does not track `{name}`; {hint}"));
        assert!(
            recorded.hash == component.hash().unwrap(),
            "components.toml records a stale hash for `{name}`; {hint}",
        );
    }

    for name in tracked.keys() {
        assert!(
            registry.get(name).is_some(),
            "components.toml tracks `{name}`, which the registry no longer offers; \
             run `topcoat ui remove {name}` in examples/ui",
        );
    }
}
