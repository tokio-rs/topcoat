use std::collections::{BTreeMap, BTreeSet};

use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, value};

use super::name::PackageName;

/// A dependency requirement in a generated manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    version: &'static str,
    default_features: bool,
    features: BTreeSet<&'static str>,
}

impl Dependency {
    /// Requires `version` with the dependency's default features.
    pub fn new(version: &'static str) -> Self {
        Self {
            version,
            default_features: true,
            features: BTreeSet::new(),
        }
    }

    /// Disables the dependency's default features.
    pub fn no_default_features(mut self) -> Self {
        self.default_features = false;
        self
    }

    /// Enables `features` in addition to those already enabled.
    pub fn features(mut self, features: impl IntoIterator<Item = &'static str>) -> Self {
        self.features.extend(features);
        self
    }

    /// Combines another requirement on the same crate by enabling the features of both.
    /// Fails if the requirements differ in version or default features.
    fn merge(&mut self, name: &str, other: Self) -> Result<(), String> {
        if self.version != other.version || self.default_features != other.default_features {
            return Err(format!(
                "conflicting requirements on dependency `{name}`: {self:?} and {other:?}"
            ));
        }
        self.features.extend(other.features);
        Ok(())
    }

    fn to_item(&self) -> Item {
        if self.default_features && self.features.is_empty() {
            return value(self.version);
        }
        let mut table = InlineTable::new();
        table.insert("version", self.version.into());
        if !self.default_features {
            table.insert("default-features", false.into());
        }
        if !self.features.is_empty() {
            table.insert(
                "features",
                Array::from_iter(self.features.iter().copied()).into(),
            );
        }
        value(table)
    }
}

/// The `Cargo.toml` of a new application.
pub struct Manifest {
    name: PackageName,
    dependencies: BTreeMap<&'static str, Dependency>,
    build_dependencies: BTreeMap<&'static str, Dependency>,
}

impl Manifest {
    pub fn new(name: PackageName) -> Self {
        Self {
            name,
            dependencies: BTreeMap::new(),
            build_dependencies: BTreeMap::new(),
        }
    }

    /// Adds a normal dependency, merging it with an earlier requirement on the same
    /// crate.
    pub fn dependency(&mut self, name: &'static str, dependency: Dependency) -> Result<(), String> {
        add(&mut self.dependencies, name, dependency)
    }

    /// Adds a build dependency, merging it with an earlier requirement on the same
    /// crate.
    pub fn build_dependency(
        &mut self,
        name: &'static str,
        dependency: Dependency,
    ) -> Result<(), String> {
        add(&mut self.build_dependencies, name, dependency)
    }

    /// Renders the manifest. Dependencies and features are sorted by name.
    pub fn render(&self) -> String {
        let mut document = DocumentMut::new();

        let mut package = Table::new();
        package["name"] = value(self.name.as_str());
        package["version"] = value("0.1.0");
        package["edition"] = value("2024");
        document["package"] = Item::Table(package);

        for (key, dependencies) in [
            ("dependencies", &self.dependencies),
            ("build-dependencies", &self.build_dependencies),
        ] {
            if dependencies.is_empty() {
                continue;
            }
            let mut table = Table::new();
            for (name, dependency) in dependencies {
                table[name] = dependency.to_item();
            }
            table.decor_mut().set_prefix("\n");
            document[key] = Item::Table(table);
        }

        let mut dev = Table::new();
        dev.decor_mut()
            .set_prefix("\n# Speed up compilation while keeping file and line numbers in error backtraces.\n");
        dev["debug"] = value("line-tables-only");

        let mut dependencies = Table::new();
        dependencies
            .decor_mut()
            .set_prefix("\n# Speed up compilation by skipping debug information for dependencies.\n");
        dependencies["debug"] = value(false);

        let mut packages = Table::new();
        packages.set_implicit(true);
        packages["*"] = Item::Table(dependencies);
        dev["package"] = Item::Table(packages);

        let mut profiles = Table::new();
        profiles.set_implicit(true);
        profiles["dev"] = Item::Table(dev);
        document["profile"] = Item::Table(profiles);

        document.to_string()
    }
}

fn add(
    dependencies: &mut BTreeMap<&'static str, Dependency>,
    name: &'static str,
    dependency: Dependency,
) -> Result<(), String> {
    if let Some(existing) = dependencies.get_mut(name) {
        existing.merge(name, dependency)
    } else {
        dependencies.insert(name, dependency);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        Manifest::new(PackageName::new("my-app").unwrap())
    }

    fn parse(manifest: &Manifest) -> toml::Table {
        manifest.render().parse().unwrap()
    }

    #[test]
    fn merges_features_of_repeated_requirements() {
        let mut manifest = manifest();
        let topcoat = || Dependency::new("1.2.3").no_default_features();
        manifest
            .dependency("topcoat", topcoat().features(["router", "view"]))
            .unwrap();
        manifest
            .dependency("topcoat", topcoat().features(["asset", "router"]))
            .unwrap();

        let parsed = parse(&manifest);
        let topcoat = &parsed["dependencies"]["topcoat"];
        assert_eq!(topcoat["version"].as_str(), Some("1.2.3"));
        assert_eq!(topcoat["default-features"].as_bool(), Some(false));
        let features: Vec<&str> = topcoat["features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|feature| feature.as_str().unwrap())
            .collect();
        assert_eq!(features, ["asset", "router", "view"]);
    }

    #[test]
    fn rejects_incompatible_requirements() {
        let mut manifest = manifest();
        manifest.dependency("tokio", Dependency::new("1")).unwrap();
        assert!(manifest.dependency("tokio", Dependency::new("2")).is_err());
        assert!(
            manifest
                .dependency("tokio", Dependency::new("1").no_default_features())
                .is_err()
        );
    }

    #[test]
    fn keeps_normal_and_build_dependencies_apart() {
        let mut manifest = manifest();
        manifest.dependency("tokio", Dependency::new("1")).unwrap();
        manifest
            .build_dependency("topcoat", Dependency::new("1").no_default_features())
            .unwrap();

        let parsed = parse(&manifest);
        assert_eq!(parsed["package"]["name"].as_str(), Some("my-app"));
        assert!(parsed["dependencies"].get("topcoat").is_none());
        assert!(parsed["build-dependencies"].get("tokio").is_none());
        assert!(parsed["build-dependencies"].get("topcoat").is_some());
    }
}
