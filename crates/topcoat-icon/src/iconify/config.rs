use std::{
    env,
    fmt::Display,
    fs, io,
    io::Read as _,
    path::{Path, PathBuf},
};

use crate::iconify::{BuildError, IconSet, Result, set::STAGE_DIR};

/// Builder for staging the Iconify icon sets used by the
/// `iconify::include!` and `iconify::iconify_icon!` macros. Use from a
/// build script:
///
/// ```rust,no_run
/// topcoat::icon::iconify::BuildConfig::new()
///     .icon_set("lucide")
///     .icon_set_version("mdi", "1.30.0")
///     .stage()
///     .unwrap();
/// ```
///
/// Each set is staged to `$OUT_DIR/topcoat-icon-iconify/<set>.json` in the
/// `IconifyJSON` format, downloading the set's `@iconify-json/<set>` package
/// from jsDelivr into a cache first. Once a set is cached, builds stay
/// offline.
///
/// The cache lives in `topcoat/cache/iconify` inside the Cargo target
/// directory by default, shared across the workspace; pass a
/// [`cache_dir`](Self::cache_dir) to use a directory of your own instead.
/// Files you place in the cache yourself are picked up without downloading,
/// so icon sets that are not on Iconify can be vendored the same way.
#[derive(Debug, Default)]
pub struct BuildConfig {
    cache_dir: Option<PathBuf>,
    sets: Vec<Set>,
}

impl BuildConfig {
    /// Creates a configuration without any sets.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stages the latest version of `set`, downloading it when it is not
    /// cached yet. To pick up new releases, delete the cached copy or pin a
    /// version with [`icon_set_version`](Self::icon_set_version).
    #[must_use]
    pub fn icon_set(mut self, set: impl Into<String>) -> Self {
        self.sets.push(Set {
            name: set.into(),
            version: None,
        });
        self
    }

    /// Stages `version` of `set`, downloading it whenever the cached copy
    /// was cached from a different version. The version a copy was cached
    /// from is tracked in a `<set>.version` file next to it.
    #[must_use]
    pub fn icon_set_version(mut self, set: impl Into<String>, version: impl Into<String>) -> Self {
        self.sets.push(Set {
            name: set.into(),
            version: Some(version.into()),
        });
        self
    }

    /// Caches downloaded sets in `dir` instead of the shared Topcoat cache
    /// in the target directory. Relative paths resolve against
    /// `CARGO_MANIFEST_DIR` (the package root).
    ///
    /// Each set is cached at `<dir>/<set>.json` and downloaded only when
    /// its file is missing or, for an
    /// [`icon_set_version`](Self::icon_set_version) set, cached from a
    /// different version. Files you place there yourself are used as-is.
    /// Commit the directory for offline, reproducible builds, or gitignore
    /// it to keep a cache that survives `cargo clean`.
    #[must_use]
    pub fn cache_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cache_dir = Some(dir.into());
        self
    }

    /// Stages every set into `$OUT_DIR/topcoat-icon-iconify`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `OUT_DIR` is unset (or `CARGO_MANIFEST_DIR`, with a
    /// cache directory), a download fails, a file cannot be read or written,
    /// or a set does not match the `IconifyJSON` schema, declares a prefix
    /// other than the name it is staged as, or contains an alias that does
    /// not lead to an icon.
    pub fn stage(self) -> Result {
        let out_dir = env::var_os("OUT_DIR").ok_or(BuildError::NoOutDir)?;
        let dir = PathBuf::from(out_dir).join(STAGE_DIR);
        fs::create_dir_all(&dir).map_err(|source| BuildError::Io {
            path: dir.clone(),
            source,
        })?;

        let cache_dir = match self.cache_dir {
            Some(cache_dir) => {
                let manifest_dir =
                    env::var_os("CARGO_MANIFEST_DIR").ok_or(BuildError::NoManifestDir)?;
                Some(PathBuf::from(manifest_dir).join(cache_dir))
            }
            // An `OUT_DIR` outside Cargo's layout has no shared cache; fall
            // back to caching inside the stage directory itself.
            None => topcoat_core::cache::cache_dir("iconify"),
        };

        for set in self.sets {
            set.stage(&dir, cache_dir.as_deref())?;
        }

        Ok(())
    }
}

/// One icon set to stage: the name of its `@iconify-json/<name>` package and
/// an optionally pinned version.
#[derive(Debug)]
struct Set {
    name: String,
    version: Option<String>,
}

impl Set {
    /// Stages this set into `dir`, downloading it through the cache at
    /// `cache_dir`, or through `dir` itself when no cache directory is
    /// configured.
    ///
    /// The cached copy is reused unless the set is pinned to a version other
    /// than the one the copy was cached from: a latest set never goes stale,
    /// keeping builds offline.
    fn stage(self, dir: &Path, cache_dir: Option<&Path>) -> Result {
        let cache = cache_dir.unwrap_or(dir);
        let cached = cache.join(format!("{}.json", self.name));
        let sidecar = cache.join(format!("{}.version", self.name));

        let fresh = cached.exists()
            && match &self.version {
                Some(version) => {
                    fs::read_to_string(&sidecar).ok().as_deref() == Some(version.as_str())
                }
                None => true,
            };

        if !fresh {
            let bytes = self.fetch()?;
            fs::create_dir_all(cache).map_err(|source| BuildError::Io {
                path: cache.to_path_buf(),
                source,
            })?;
            write(&cached, &bytes)?;
            match &self.version {
                Some(version) => write(&sidecar, version.as_bytes())?,
                // Drop a sidecar left over from a previously pinned
                // download; the cached copy no longer matches it.
                None => match fs::remove_file(&sidecar) {
                    Err(source) if source.kind() != io::ErrorKind::NotFound => {
                        return Err(BuildError::Io {
                            path: sidecar,
                            source,
                        });
                    }
                    _ => {}
                },
            }
        }

        // A separate cache directory is user-managed: check its file and
        // copy it into the stage directory. The stage directory itself is
        // not edited by hand, so its files are staged as they are.
        if cache_dir.is_some() {
            let bytes = fs::read(&cached).map_err(|source| BuildError::Io {
                path: cached.clone(),
                source,
            })?;
            self.check_prefix(&parse(&bytes, cached.display())?)?;
            write(&dir.join(format!("{}.json", self.name)), &bytes)?;
        }

        Ok(())
    }

    /// Fetches this set from jsDelivr, at the pinned version or the `latest`
    /// tag, checking that the payload parses and declares the requested
    /// prefix.
    fn fetch(&self) -> Result<Vec<u8>> {
        let tag = self.version.as_deref().unwrap_or("latest");
        let url = format!(
            "https://cdn.jsdelivr.net/npm/@iconify-json/{name}@{tag}/icons.json",
            name = self.name,
        );
        let bytes = download(&url)?;
        self.check_prefix(&parse(&bytes, &url)?)?;
        Ok(bytes)
    }

    /// Checks that a cached or downloaded set declares the prefix it is
    /// staged as, which is also the name its icons are addressed by.
    fn check_prefix(&self, parsed: &IconSet) -> Result {
        if parsed.prefix != self.name {
            return Err(BuildError::PrefixMismatch {
                requested: self.name.clone(),
                declared: parsed.prefix.clone(),
            });
        }
        Ok(())
    }
}

/// Fetches `url` into memory.
fn download(url: &str) -> Result<Vec<u8>> {
    let http_error = |source: ureq::Error| BuildError::Http {
        url: url.to_owned(),
        source: Box::new(source),
    };

    let mut body = ureq::get(url).call().map_err(http_error)?.into_body();
    let mut bytes = Vec::new();
    body.as_reader()
        .read_to_end(&mut bytes)
        .map_err(|source| http_error(source.into()))?;
    Ok(bytes)
}

/// Parses and checks a set: the JSON must match the `IconifyJSON` schema and
/// every alias must lead to an icon, so problems surface while staging
/// rather than in a macro expansion.
fn parse(bytes: &[u8], origin: impl Display) -> Result<IconSet> {
    let set: IconSet = serde_json::from_slice(bytes).map_err(|source| BuildError::Json {
        origin: origin.to_string(),
        source,
    })?;

    for (name, alias) in &set.aliases {
        let mut parent = &alias.parent;
        let mut steps = 0;
        while !set.icons.contains_key(parent) {
            let Some(next) = set.aliases.get(parent) else {
                return Err(BuildError::UnknownAliasParent {
                    prefix: set.prefix.clone(),
                    alias: name.clone(),
                    parent: parent.clone(),
                });
            };
            steps += 1;
            if steps > set.aliases.len() {
                return Err(BuildError::AliasCycle {
                    prefix: set.prefix.clone(),
                    alias: name.clone(),
                });
            }
            parent = &next.parent;
        }
    }

    Ok(set)
}

/// Writes `bytes` to `path`, wrapping errors with the path.
fn write(path: &Path, bytes: &[u8]) -> Result {
    fs::write(path, bytes).map_err(|source| BuildError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh directory under the system temp directory.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("topcoat-icon-iconify-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn set(name: &str, version: Option<&str>) -> Set {
        Set {
            name: name.to_owned(),
            version: version.map(str::to_owned),
        }
    }

    const DEMO: &[u8] = br#"{ "prefix": "demo", "icons": { "trash": { "body": "<g/>" } } }"#;

    #[test]
    fn cached_files_stage_offline() {
        let root = temp_dir("cached-files-stage");
        let cache_dir = root.join("icons");
        let stage_dir = root.join("staged");
        fs::create_dir_all(&cache_dir).unwrap();
        fs::create_dir_all(&stage_dir).unwrap();

        // A fetch of the nonexistent `demo` package would fail.
        fs::write(cache_dir.join("demo.json"), DEMO).unwrap();
        set("demo", None)
            .stage(&stage_dir, Some(&cache_dir))
            .unwrap();

        assert_eq!(fs::read(stage_dir.join("demo.json")).unwrap(), DEMO);
    }

    #[test]
    fn pinned_sets_reuse_copies_of_the_same_version() {
        let root = temp_dir("pinned-sets-reuse");
        let cache_dir = root.join("icons");
        let stage_dir = root.join("staged");
        fs::create_dir_all(&cache_dir).unwrap();
        fs::create_dir_all(&stage_dir).unwrap();

        fs::write(cache_dir.join("demo.json"), DEMO).unwrap();
        fs::write(cache_dir.join("demo.version"), "9.9.9").unwrap();
        set("demo", Some("9.9.9"))
            .stage(&stage_dir, Some(&cache_dir))
            .unwrap();

        assert_eq!(fs::read(stage_dir.join("demo.json")).unwrap(), DEMO);
    }

    #[test]
    fn the_build_dir_cache_reuses_files_as_they_are() {
        let root = temp_dir("build-dir-reuse");
        let stage_dir = root.join("staged");
        fs::create_dir_all(&stage_dir).unwrap();

        fs::write(stage_dir.join("demo.json"), DEMO).unwrap();
        set("demo", None).stage(&stage_dir, None).unwrap();

        assert_eq!(fs::read(stage_dir.join("demo.json")).unwrap(), DEMO);
    }

    #[test]
    fn cached_files_must_declare_their_prefix() {
        let root = temp_dir("cached-files-prefix");
        let cache_dir = root.join("icons");
        let stage_dir = root.join("staged");
        fs::create_dir_all(&cache_dir).unwrap();
        fs::create_dir_all(&stage_dir).unwrap();

        let json = br#"{ "prefix": "other", "icons": { "trash": { "body": "<g/>" } } }"#;
        fs::write(cache_dir.join("demo.json"), json).unwrap();

        let error = set("demo", None)
            .stage(&stage_dir, Some(&cache_dir))
            .unwrap_err();
        assert!(
            matches!(
                &error,
                BuildError::PrefixMismatch { requested, declared }
                    if requested == "demo" && declared == "other"
            ),
            "{error}"
        );
    }

    #[test]
    fn parse_reports_dangling_alias_parents() {
        let json = br#"{
            "prefix": "demo",
            "icons": { "trash": { "body": "<g/>" } },
            "aliases": { "bin": { "parent": "rubbish" } }
        }"#;

        let error = parse(json, "test").unwrap_err();
        assert!(
            matches!(
                &error,
                BuildError::UnknownAliasParent { prefix, alias, parent }
                    if prefix == "demo" && alias == "bin" && parent == "rubbish"
            ),
            "{error}"
        );
    }

    #[test]
    fn parse_reports_alias_cycles() {
        let json = br#"{
            "prefix": "demo",
            "icons": { "trash": { "body": "<g/>" } },
            "aliases": {
                "yin": { "parent": "yang" },
                "yang": { "parent": "yin" }
            }
        }"#;

        let error = parse(json, "test").unwrap_err();
        assert!(
            matches!(&error, BuildError::AliasCycle { prefix, .. } if prefix == "demo"),
            "{error}"
        );
    }

    #[test]
    fn parse_accepts_alias_chains() {
        let json = br#"{
            "prefix": "demo",
            "icons": { "trash": { "body": "<g/>" } },
            "aliases": {
                "bin": { "parent": "trash" },
                "wastebasket": { "parent": "bin" }
            }
        }"#;

        assert!(parse(json, "test").is_ok());
    }
}
