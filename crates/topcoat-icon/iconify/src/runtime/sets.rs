use std::{
    env,
    fmt::Display,
    fs,
    io::Read as _,
    path::{Path, PathBuf},
};

use crate::runtime::{BuildError, IconSet, Result, set::STAGE_DIR};

/// The Iconify icon sets to stage for the `iconify::include!` and
/// `iconify::iconify_icon!` macros. Use from a build script:
///
/// ```rust,no_run
/// topcoat::icon::iconify::Sets::new()
///     .download("lucide")
///     .download_version("mdi", "1.30.0")
///     .vendor("vendor/simple-icons.json")
///     .stage()
///     .unwrap();
/// ```
///
/// Each set is written to `$OUT_DIR/topcoat-icon-iconify/<set>.json` in the
/// `IconifyJSON` format, exactly as downloaded or vendored. Downloads fetch
/// the set's `@iconify-json/<set>` package from jsDelivr; once a set is
/// staged, builds stay offline.
#[derive(Debug, Default)]
pub struct Sets {
    sources: Vec<Source>,
}

impl Sets {
    /// Creates an empty collection of sets.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stages the latest version of `set`, downloading it on the first
    /// build and reusing the staged copy afterwards. To pick up new releases,
    /// clean the package's build directory or pin a version with
    /// [`download_version`](Self::download_version).
    #[must_use]
    pub fn download(mut self, set: impl Into<String>) -> Self {
        self.sources.push(Source::Download {
            set: set.into(),
            version: None,
        });
        self
    }

    /// Stages `version` of `set`, downloading it whenever the staged copy
    /// was built from a different version.
    #[must_use]
    pub fn download_version(mut self, set: impl Into<String>, version: impl Into<String>) -> Self {
        self.sources.push(Source::Download {
            set: set.into(),
            version: Some(version.into()),
        });
        self
    }

    /// Stages a local Iconify JSON file, named after the `prefix` it
    /// declares. Relative paths resolve against `CARGO_MANIFEST_DIR` (the
    /// package root).
    #[must_use]
    pub fn vendor(mut self, path: impl Into<PathBuf>) -> Self {
        self.sources.push(Source::Vendor { path: path.into() });
        self
    }

    /// Stages every set into `$OUT_DIR/topcoat-icon-iconify`.
    ///
    /// Prints `cargo::rerun-if-changed` directives for the build script
    /// itself and every vendored file, so vendored edits restage while
    /// ordinary source edits do not rerun the script.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `OUT_DIR` is unset (or `CARGO_MANIFEST_DIR`, with
    /// vendored sets), a download fails, a file cannot be read or written,
    /// or a set does not match the `IconifyJSON` schema, declares a prefix
    /// other than the name it was downloaded as, or contains an alias that
    /// does not lead to an icon.
    pub fn stage(self) -> Result {
        println!("cargo::rerun-if-changed=build.rs");

        let out_dir = env::var_os("OUT_DIR").ok_or(BuildError::NoOutDir)?;
        let dir = PathBuf::from(out_dir).join(STAGE_DIR);
        fs::create_dir_all(&dir).map_err(|source| BuildError::Io {
            path: dir.clone(),
            source,
        })?;

        for source in self.sources {
            source.stage(&dir)?;
        }

        Ok(())
    }
}

/// Where one staged icon set comes from.
#[derive(Debug)]
enum Source {
    /// The set's `@iconify-json/<set>` package on jsDelivr, at a pinned
    /// version or the `latest` tag.
    Download {
        set: String,
        version: Option<String>,
    },
    /// A local Iconify JSON file.
    Vendor { path: PathBuf },
}

impl Source {
    /// Stages this source into `dir`, skipping downloads that are already
    /// staged.
    fn stage(self, dir: &Path) -> Result {
        match self {
            Self::Vendor { path } => {
                let manifest_dir =
                    env::var_os("CARGO_MANIFEST_DIR").ok_or(BuildError::NoManifestDir)?;
                let path = PathBuf::from(manifest_dir).join(path);
                println!("cargo::rerun-if-changed={}", path.display());

                let bytes = fs::read(&path).map_err(|source| BuildError::Io {
                    path: path.clone(),
                    source,
                })?;
                let set = parse(&bytes, path.display())?;
                write(&dir.join(format!("{}.json", set.prefix)), &bytes)
            }
            Self::Download { set, version } => {
                let staged = dir.join(format!("{set}.json"));
                let sidecar = dir.join(format!("{set}.version"));
                let staged_version = || fs::read_to_string(&sidecar).ok();
                let skip = match &version {
                    // A pinned set is fresh when it was staged from the same
                    // version.
                    Some(version) => {
                        staged.exists() && staged_version().as_deref() == Some(version)
                    }
                    // A latest set never goes stale, keeping builds offline.
                    None => staged.exists(),
                };
                if skip {
                    return Ok(());
                }

                let tag = version.as_deref().unwrap_or("latest");
                let url =
                    format!("https://cdn.jsdelivr.net/npm/@iconify-json/{set}@{tag}/icons.json");
                let bytes = download(&url)?;
                let parsed = parse(&bytes, &url)?;
                if parsed.prefix != set {
                    return Err(BuildError::PrefixMismatch {
                        requested: set,
                        declared: parsed.prefix,
                    });
                }

                write(&staged, &bytes)?;
                match version {
                    Some(version) => write(&sidecar, version.as_bytes()),
                    // Drop a sidecar left over from a previously pinned
                    // download; the staged copy no longer matches it.
                    None => match fs::remove_file(&sidecar) {
                        Err(source) if source.kind() != std::io::ErrorKind::NotFound => {
                            Err(BuildError::Io {
                                path: sidecar,
                                source,
                            })
                        }
                        _ => Ok(()),
                    },
                }
            }
        }
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
