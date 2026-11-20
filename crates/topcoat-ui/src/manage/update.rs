use std::{collections::HashMap, io::ErrorKind, path::PathBuf};

use super::{
    merge::{LineEnding, merge, normalize},
    package::Package,
    state::InstallState,
    vcs::Vcs,
    workspace::Workspace,
};
use crate::{Registry, content_hash};

/// Options for updating installed files.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is an independent switch"
)]
pub struct UpdateOptions {
    /// Names of the components to update. If none are named and `theme` is `false`,
    /// the theme and every installed component are updated.
    pub components: Vec<String>,
    /// Update the theme, alone or along with the named components.
    pub theme: bool,
    /// Plan the update and report it without writing any file.
    pub dry_run: bool,
    /// Merge even if the package's git repository has uncommitted changes.
    pub allow_dirty: bool,
    /// Merge even if the package is not in a git repository.
    pub allow_no_vcs: bool,
}

/// What [`update`] did with one installed file.
pub struct UpdateEntry {
    /// The name of the component or theme.
    pub name: String,
    /// Whether the file is a component or the theme.
    pub kind: UpdateKind,
    /// The package-relative path of the file.
    pub file: PathBuf,
    /// The registry the file was installed from.
    pub registry: String,
    /// What happened to the file.
    pub action: UpdateAction,
}

/// The kind of file an [`UpdateEntry`] describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateKind {
    Theme,
    Component,
}

impl UpdateKind {
    /// Reads the source `registry` offers under `name`, or `None` if it offers none.
    fn read(self, registry: &Registry, name: &str) -> Result<Option<String>, String> {
        let source = match self {
            Self::Theme => registry.theme(name).map(|theme| theme.read_source()),
            Self::Component => registry.get(name).map(|component| component.read_source()),
        };
        source
            .transpose()
            .map_err(|error| format!("failed to read `{name}`: {error}"))
    }
}

/// The action [`update`] took for a file.
#[derive(Debug, PartialEq, Eq)]
pub enum UpdateAction {
    /// The registry's source has not changed since the file was installed.
    UpToDate,
    /// The registry's source has not changed since the file was installed, and the
    /// install state gained the registry version it lacked.
    VersionRecorded,
    /// The file had no local changes and was replaced with the registry's new source.
    FastForwarded,
    /// The file already matched the registry's new source, so only the install state
    /// changed.
    Matched,
    /// Local changes and the registry's changes were combined without conflicts.
    Merged,
    /// The file was merged and contains conflict markers to resolve by hand.
    Conflicted {
        /// Why the source the file was installed from could not be used. Without it,
        /// every difference between the file and the registry's source is a conflict.
        missing_base: Option<String>,
    },
    /// The file was left alone, for the given reason.
    Skipped { reason: String },
}

/// Updates installed components and the theme to the registry's current sources.
///
/// A file without local changes is replaced. A file with local changes is merged with
/// the registry's changes, and overlapping changes are left in the file as conflict
/// markers. The install state then records the new source, also for files with
/// conflicts.
///
/// A file recorded without a registry version gains the current one if the
/// registry's source has not changed since the file was installed. A later update can
/// then read that source to merge.
///
/// A merge rewrites a file that holds local changes. To keep that undoable, no file
/// is written if any file needs merging while the package's git repository has
/// uncommitted changes or the package is in none. `allow_dirty` and `allow_no_vcs` in
/// `options` lift this.
///
/// Every file is planned before the first one is written. A later I/O failure can
/// leave some files updated and others not.
///
/// Returns an entry for every selected file.
///
/// # Errors
///
/// Returns an error if the install state or workspace cannot be loaded, a named
/// component is not installed, the theme is requested but none is installed, a merge
/// is refused as described above, or reading or writing a file or saving the install
/// state fails.
pub fn update(package: &Package, options: &UpdateOptions) -> Result<Vec<UpdateEntry>, String> {
    // Read before the workspace is loaded, because resolving it can rewrite
    // `Cargo.lock`, which must not count as an uncommitted change.
    let vcs = Vcs::inspect(package.root());
    let mut state = InstallState::load(package)?;
    let workspace = Workspace::load(package)?;
    let targets = Target::select(&state, options)?;

    let mut sources = Sources {
        package,
        workspace: &workspace,
        current: HashMap::new(),
        published: HashMap::new(),
    };
    let mut entries = Vec::with_capacity(targets.len());
    let mut writes = Vec::new();
    let mut changed = false;

    for target in targets {
        let plan = sources.plan(&target)?;
        let mut action = plan.action;
        let version = workspace
            .registry_version(&target.registry)
            .map(str::to_string);
        if let Some(hash) = plan.hash {
            target.record(&mut state, hash, version);
            changed = true;
        } else if action == UpdateAction::UpToDate && target.version.is_none() && version.is_some()
        {
            // The recorded hash is that of the current source, so the current version
            // of the registry holds the source the file was installed from.
            target.record(&mut state, target.hash.clone(), version);
            action = UpdateAction::VersionRecorded;
            changed = true;
        }
        if let Some(contents) = plan.contents {
            writes.push((package.resolve(&target.file), contents));
        }
        entries.push(UpdateEntry {
            name: target.name,
            kind: target.kind,
            file: target.file,
            registry: target.registry,
            action,
        });
    }

    if !options.dry_run {
        let merges = entries.iter().any(|entry| {
            matches!(
                entry.action,
                UpdateAction::Merged | UpdateAction::Conflicted { .. }
            )
        });
        if merges {
            vcs.ensure_undoable(options.allow_dirty, options.allow_no_vcs)?;
        }
        for (file, contents) in &writes {
            std::fs::write(file, contents)
                .map_err(|error| format!("failed to write {}: {error}", file.display()))?;
        }
        if changed {
            state.save(package)?;
        }
    }
    Ok(entries)
}

/// An installed file selected for updating, with what the install state records for
/// it.
struct Target {
    kind: UpdateKind,
    registry: String,
    name: String,
    hash: String,
    version: Option<String>,
    file: PathBuf,
}

impl Target {
    /// Resolves the files `options` selects.
    fn select(state: &InstallState, options: &UpdateOptions) -> Result<Vec<Self>, String> {
        let everything = options.components.is_empty() && !options.theme;
        let mut targets = Vec::new();

        if everything || options.theme {
            match &state.theme {
                Some(theme) => targets.push(Self {
                    kind: UpdateKind::Theme,
                    registry: theme.registry.clone(),
                    name: theme.name.clone(),
                    hash: theme.hash.clone(),
                    version: theme.version.clone(),
                    file: theme.file.clone(),
                }),
                None if options.theme => return Err(String::from("no theme is installed")),
                None => {}
            }
        }

        let before = targets.len();
        for (registry, tracked) in &state.registries {
            for (name, component) in &tracked.components {
                if everything || options.components.contains(name) {
                    targets.push(Self {
                        kind: UpdateKind::Component,
                        registry: registry.clone(),
                        name: name.clone(),
                        hash: component.hash.clone(),
                        version: component.version.clone(),
                        file: component.file.clone(),
                    });
                }
            }
        }
        let selected = &targets[before..];
        if let Some(missing) = options
            .components
            .iter()
            .find(|name| !selected.iter().any(|target| &target.name == *name))
        {
            return Err(format!(
                "component `{missing}` is not installed; run `topcoat ui add {missing}` to install it"
            ));
        }

        Ok(targets)
    }

    /// Records in `state` that the file now derives from the registry source with
    /// `hash`, published in `version`.
    fn record(&self, state: &mut InstallState, hash: String, version: Option<String>) {
        match self.kind {
            UpdateKind::Theme => {
                if let Some(theme) = &mut state.theme {
                    theme.hash = hash;
                    theme.version = version;
                }
            }
            UpdateKind::Component => {
                if let Some(component) = state
                    .registry_mut(&self.registry)
                    .components
                    .get_mut(&self.name)
                {
                    component.hash = hash;
                    component.version = version;
                }
            }
        }
    }
}

/// Reads registry sources, loading each registry once.
struct Sources<'a> {
    package: &'a Package,
    workspace: &'a Workspace,
    /// Registries as the package currently resolves them, by name.
    current: HashMap<String, Result<Registry, String>>,
    /// Registries as published in a version of their crate, by name and version.
    published: HashMap<(String, String), Result<Registry, String>>,
}

impl Sources<'_> {
    /// Plans the update of one installed file.
    fn plan(&mut self, target: &Target) -> Result<Plan, String> {
        let workspace = self.workspace;
        let current = self
            .current
            .entry(target.registry.clone())
            .or_insert_with(|| {
                let dir = workspace.registry_dir(&target.registry)?;
                Registry::load(dir).map_err(|error| {
                    format!("failed to load registry `{}`: {error}", target.registry)
                })
            });
        let theirs = match current {
            Ok(registry) => match target.kind.read(registry, &target.name)? {
                Some(theirs) => theirs,
                None => {
                    return Ok(Plan::skip(format!(
                        "registry `{}` no longer offers it",
                        target.registry
                    )));
                }
            },
            Err(error) => return Ok(Plan::skip(error.clone())),
        };

        let path = self.package.resolve(&target.file);
        let ours = match std::fs::read_to_string(&path) {
            Ok(ours) => ours,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(Plan::skip(String::from("the file is missing")));
            }
            Err(error) => return Err(format!("failed to read {}: {error}", path.display())),
        };

        Ok(Plan::new(&target.hash, &ours, &theirs, || {
            self.base(target)
        }))
    }

    /// Reads the source `target` was installed from, out of the registry version the
    /// install state records for it.
    fn base(&mut self, target: &Target) -> Result<String, String> {
        let version = target
            .version
            .as_deref()
            .ok_or("no registry version is recorded for it")?;
        let (package, workspace) = (self.package, self.workspace);
        let published = self
            .published
            .entry((target.registry.clone(), version.to_string()))
            .or_insert_with(|| workspace.published_registry(package, &target.registry, version))
            .as_ref()
            .map_err(Clone::clone)?;

        let base = target
            .kind
            .read(published, &target.name)?
            .ok_or_else(|| format!("version {version} of the registry does not offer it"))?;
        if content_hash(&base) != target.hash {
            return Err(format!(
                "the source in version {version} of the registry does not match the recorded hash"
            ));
        }
        Ok(base)
    }
}

/// What to do with one installed file.
struct Plan {
    action: UpdateAction,
    /// The new contents of the file, if it changes.
    contents: Option<String>,
    /// The hash of the registry source the file derives from afterwards, if that
    /// changes.
    hash: Option<String>,
}

impl Plan {
    /// Decides what to do with a file.
    ///
    /// `recorded` is the hash of the registry source the file was installed from, `ours`
    /// is the file, and `theirs` is the registry's current source. `base` reads the
    /// source the file was installed from, or explains why it is unavailable. It is
    /// only called when both the file and the registry's source have changed.
    fn new(
        recorded: &str,
        ours: &str,
        theirs: &str,
        base: impl FnOnce() -> Result<String, String>,
    ) -> Self {
        let latest = content_hash(theirs);
        if latest == recorded {
            return Self {
                action: UpdateAction::UpToDate,
                contents: None,
                hash: None,
            };
        }

        let normalized = normalize(ours);
        if normalized == normalize(theirs) {
            return Self {
                action: UpdateAction::Matched,
                contents: None,
                hash: Some(latest),
            };
        }

        // The recorded hash is compared with and without normalizing, because the
        // registry source it was taken from may itself have had CRLF line endings.
        if content_hash(&normalized) == recorded || content_hash(ours) == recorded {
            let contents = LineEnding::detect(ours).apply(normalize(theirs).into_owned());
            return Self {
                action: UpdateAction::FastForwarded,
                contents: Some(contents),
                hash: Some(latest),
            };
        }

        let base = base();
        let merged = merge(base.as_deref().ok(), ours, theirs);
        let action = match (merged.conflicted, base) {
            (false, _) => UpdateAction::Merged,
            (true, Ok(_)) => UpdateAction::Conflicted { missing_base: None },
            (true, Err(reason)) => UpdateAction::Conflicted {
                missing_base: Some(reason),
            },
        };
        Self {
            action,
            contents: Some(merged.text),
            hash: Some(latest),
        }
    }

    /// Leaves the file alone for `reason`.
    fn skip(reason: String) -> Self {
        Self {
            action: UpdateAction::Skipped { reason },
            contents: None,
            hash: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INSTALLED: &str = "one\ntwo\nthree\nfour\nfive\n";

    /// A base that must not be needed.
    fn unused() -> Result<String, String> {
        panic!("the installed source should not be read")
    }

    #[test]
    fn unchanged_registry_source_leaves_a_modified_file_alone() {
        let ours = INSTALLED.replace("two", "TWO");

        let plan = Plan::new(&content_hash(INSTALLED), &ours, INSTALLED, unused);

        assert_eq!(plan.action, UpdateAction::UpToDate);
        assert_eq!(plan.contents, None);
        assert_eq!(plan.hash, None);
    }

    #[test]
    fn unmodified_file_takes_the_new_source() {
        let theirs = INSTALLED.replace("four", "FOUR");

        let plan = Plan::new(&content_hash(INSTALLED), INSTALLED, &theirs, unused);

        assert_eq!(plan.action, UpdateAction::FastForwarded);
        assert_eq!(plan.contents.as_deref(), Some(theirs.as_str()));
        assert_eq!(plan.hash, Some(content_hash(&theirs)));
    }

    #[test]
    fn modified_file_keeps_its_changes_and_gains_the_registry_changes() {
        let ours = INSTALLED.replace("two", "TWO");
        let theirs = INSTALLED.replace("four", "FOUR");

        let plan = Plan::new(&content_hash(INSTALLED), &ours, &theirs, || {
            Ok(INSTALLED.to_string())
        });

        assert_eq!(plan.action, UpdateAction::Merged);
        assert_eq!(
            plan.contents.as_deref(),
            Some("one\nTWO\nthree\nFOUR\nfive\n")
        );
        assert_eq!(plan.hash, Some(content_hash(&theirs)));
    }

    #[test]
    fn modified_file_conflicts_when_the_installed_source_is_unavailable() {
        let ours = INSTALLED.replace("two", "TWO");
        let theirs = INSTALLED.replace("four", "FOUR");

        let plan = Plan::new(&content_hash(INSTALLED), &ours, &theirs, || {
            Err(String::from("unavailable"))
        });

        assert!(matches!(
            plan.action,
            UpdateAction::Conflicted {
                missing_base: Some(_)
            }
        ));
        let contents = plan.contents.unwrap();
        assert!(contents.contains("TWO\n") && contents.contains("FOUR\n"));
    }
}
