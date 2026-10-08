use std::{collections::BTreeMap, fmt::Write as _, path::PathBuf};

use super::{
    init::ThemePlan,
    state::{InstallState, InstalledComponent, InstalledTheme, STATE_FILE},
};
use crate::{DEFAULT_REGISTRY, Dependency, Registry, content_hash};

/// Options for [`scaffold`].
pub struct ScaffoldOptions<'a> {
    /// The theme to install. If omitted, the registry must offer exactly one theme.
    pub theme: Option<&'a str>,
    /// The components to install. Their dependencies are installed too.
    pub components: &'a [&'a str],
}

/// A file planned by [`scaffold`].
pub struct ScaffoldFile {
    /// The package-relative path of the file.
    pub path: PathBuf,
    /// The file's contents.
    pub contents: String,
}

/// Plans the component files of a new package without writing anything.
///
/// `registry` is the default registry. The result holds the theme stylesheet, each
/// requested component and its dependencies, the module file declaring them, and the
/// install state, matching what `init` followed by `add` would create. Files are sorted
/// by path.
///
/// # Errors
///
/// Returns an error if the theme is unknown, no theme is named while several are
/// available, a component is unknown or depends on a component from another registry,
/// or a source cannot be read.
pub fn scaffold(
    registry: &Registry,
    options: &ScaffoldOptions<'_>,
) -> Result<Vec<ScaffoldFile>, String> {
    let mut choose = |names: &[String]| -> Result<String, String> {
        Err(format!(
            "several themes are available; name one of: {}",
            names.join(", ")
        ))
    };
    let theme = ThemePlan::select(registry, options.theme, &mut choose)?;

    let mut state = InstallState::default();
    let mut files = Vec::new();
    let mut installed: BTreeMap<String, InstalledComponent> = BTreeMap::new();
    let mut queue: Vec<String> = options
        .components
        .iter()
        .map(|name| (*name).to_string())
        .collect();

    while let Some(name) = queue.pop() {
        if installed.contains_key(&name) {
            continue;
        }
        let component = registry.get(&name).ok_or_else(|| {
            let available: Vec<&str> = registry.names().collect();
            format!(
                "unknown component `{name}` in registry `{DEFAULT_REGISTRY}`; available: {}",
                available.join(", ")
            )
        })?;
        for dependency in component.dependencies() {
            match dependency {
                Dependency::Same(dependency) => queue.push(dependency.clone()),
                Dependency::Other {
                    registry,
                    name: dependency,
                } => {
                    return Err(format!(
                        "component `{name}` depends on `{dependency}` from registry \
                         `{registry}`; only `{DEFAULT_REGISTRY}` components can be scaffolded"
                    ));
                }
            }
        }

        let contents = component
            .read_source()
            .map_err(|error| format!("failed to read component `{name}`: {error}"))?;
        let file = state.components_dir.join(component.file_name());
        installed.insert(
            name,
            InstalledComponent {
                hash: content_hash(&contents),
                file: file.clone(),
            },
        );
        files.push(ScaffoldFile {
            path: file,
            contents,
        });
    }

    // The components directory is declared by its sibling `<dir>.rs` file.
    if !installed.is_empty() {
        let mut declarations = String::new();
        for component in installed.values() {
            if let Some(module) = component.file.file_stem().and_then(|stem| stem.to_str()) {
                writeln!(declarations, "pub mod {module};")
                    .expect("writing to a String cannot fail");
            }
        }
        files.push(ScaffoldFile {
            path: state.components_dir.with_extension("rs"),
            contents: declarations,
        });
        state.registry_mut(DEFAULT_REGISTRY).components = installed;
    }

    state.theme = Some(InstalledTheme {
        name: theme.name,
        registry: theme.registry,
        hash: theme.hash,
        file: theme.file.clone(),
    });
    files.push(ScaffoldFile {
        path: theme.file,
        contents: theme.contents,
    });
    files.push(ScaffoldFile {
        path: PathBuf::from(STATE_FILE),
        contents: state.render()?,
    });

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    static FILES: &[(&str, &str)] = &[
        (
            "registry.toml",
            r#"
            version = 1

            [themes.plain]
            source = "themes/plain.css"

            [components.alert_dialog]
            source = "components/alert_dialog.rs"
            dependencies = ["dialog"]

            [components.badge]
            source = "components/badge.rs"

            [components.dialog]
            source = "components/dialog.rs"
            "#,
        ),
        ("themes/plain.css", "body { color: black; }\n"),
        ("components/alert_dialog.rs", "// alert dialog\n"),
        ("components/badge.rs", "// badge\n"),
        ("components/dialog.rs", "// dialog\n"),
    ];

    fn registry() -> Registry {
        Registry::embedded(FILES).unwrap()
    }

    fn source(path: &str) -> &'static str {
        FILES.iter().find(|(file, _)| *file == path).unwrap().1
    }

    fn file<'a>(files: &'a [ScaffoldFile], path: &str) -> Option<&'a str> {
        files
            .iter()
            .find(|file| file.path == Path::new(path))
            .map(|file| file.contents.as_str())
    }

    #[test]
    fn installs_requested_components_with_their_dependencies() {
        let options = ScaffoldOptions {
            theme: None,
            components: &["alert_dialog"],
        };
        let files = scaffold(&registry(), &options).unwrap();

        assert_eq!(
            file(&files, "src/components/alert_dialog.rs"),
            Some(source("components/alert_dialog.rs"))
        );
        assert_eq!(
            file(&files, "src/components/dialog.rs"),
            Some(source("components/dialog.rs"))
        );
        assert_eq!(file(&files, "src/components/badge.rs"), None);
        assert_eq!(file(&files, "styles.css"), Some(source("themes/plain.css")));

        let modules = file(&files, "src/components.rs").unwrap();
        let declared: Vec<&str> = modules.lines().collect();
        assert_eq!(declared, ["pub mod alert_dialog;", "pub mod dialog;"]);
    }

    #[test]
    fn install_state_tracks_every_planned_file() {
        let options = ScaffoldOptions {
            theme: Some("plain"),
            components: &["alert_dialog"],
        };
        let files = scaffold(&registry(), &options).unwrap();
        let state: InstallState = toml::from_str(file(&files, STATE_FILE).unwrap()).unwrap();

        let theme = state.theme.unwrap();
        assert_eq!(
            theme.hash,
            content_hash(file(&files, "styles.css").unwrap())
        );

        let components = &state.registries[DEFAULT_REGISTRY].components;
        assert_eq!(components.len(), 2);
        for installed in components.values() {
            let contents = file(&files, installed.file.to_str().unwrap()).unwrap();
            assert_eq!(installed.hash, content_hash(contents));
        }
    }

    #[test]
    fn rejects_unknown_components() {
        let options = ScaffoldOptions {
            theme: None,
            components: &["carousel"],
        };
        assert!(scaffold(&registry(), &options).is_err());
    }
}
