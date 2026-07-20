mod cache;
mod config;
mod error;
mod event;

use std::{
    collections::{HashMap, HashSet},
    fmt::Write as _,
    fs, io,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
};

use sha2::{Digest, Sha256};

use crate::{
    Asset, AssetError, MANIFEST_NAME, MANIFEST_VERSION, Manifest, ManifestEntry, RawAsset, Source,
};

use cache::Cache;
pub use config::*;
pub use error::*;
pub use event::*;

/// Scans a built binary for [`asset!`](crate::asset) declarations and
/// writes the referenced files into a bundle directory.
///
/// Local paths are copied; remote URLs are downloaded into `cache_dir`
/// (and reused on subsequent runs). Output filenames include a short
/// content hash, and the resulting directory is described by a
/// [`Manifest`].
pub struct Bundler {
    cache: Cache,
    parallelism: usize,
    events: BundleEvents,
}

impl Bundler {
    /// Create a bundler from a [`BundlerConfig`].
    #[must_use]
    pub fn new(config: &BundlerConfig) -> Self {
        Self {
            cache: Cache::new(config),
            parallelism: config.resolve_parallelism(),
            events: config.events().clone(),
        }
    }

    /// Scan `binary` for embedded assets and sync them into `out_dir`.
    ///
    /// If `out_dir` already contains a `manifest.toml`, it is loaded and used
    /// to skip copying files whose content hash hasn't changed. Files that
    /// were present in the old manifest but are no longer referenced by the
    /// new one are removed. Remote (http/https) assets are downloaded into
    /// the bundler's cache directory and then treated like local files.
    ///
    /// Up to [`BundlerConfig::parallelism`] assets are processed at once.
    /// The manifest lists them in declaration order regardless.
    ///
    /// This blocks on filesystem and network I/O; call it from a blocking
    /// context (e.g. [`tokio::task::spawn_blocking`]) when running inside an
    /// async runtime.
    ///
    /// # Errors
    ///
    /// Returns a [`BundleError`] for any I/O failure while creating or
    /// reading `out_dir` and its manifest, downloading a remote asset, or
    /// reading/writing a bundled file; and for a checksum mismatch between a
    /// declared asset and its configured `checksum`. When several assets
    /// fail, the one declared earliest is reported.
    pub fn bundle(&self, binary: &[u8], out_dir: impl AsRef<Path>) -> BundleResult {
        let out_dir = out_dir.as_ref();
        fs::create_dir_all(out_dir).map_err(|source| AssetError::ManifestIo {
            path: out_dir.to_path_buf(),
            source,
        })?;

        let manifest_path = out_dir.join(MANIFEST_NAME);
        let existing: HashMap<_, _> = match Manifest::load(&manifest_path) {
            Ok(manifest) => manifest
                .assets
                .into_iter()
                .map(|entry| (entry.id, entry))
                .collect(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => HashMap::new(),
            Err(source) => {
                return Err(AssetError::ManifestIo {
                    path: manifest_path,
                    source,
                }
                .into());
            }
        };

        let assets = RawAsset::find_in_binary(binary);
        self.events.emit(&BundleEvent::Scanned {
            count: assets.len(),
        });

        let mut entries = Vec::with_capacity(assets.len());
        let mut kept_files = HashSet::with_capacity(assets.len());
        let mut bundled = 0;
        let mut unchanged = 0;
        for prepared in self.prepare_all(&assets, &existing, out_dir) {
            let prepared = prepared?;
            if prepared.written {
                bundled += 1;
            } else {
                unchanged += 1;
            }
            kept_files.insert(prepared.entry.file.clone());
            entries.push(prepared.entry);
        }

        let mut removed = 0;
        for entry in existing.values() {
            if !kept_files.contains(&entry.file) {
                let path = out_dir.join(&entry.file);
                match fs::remove_file(&path) {
                    Ok(()) => {
                        removed += 1;
                        self.events.emit(&BundleEvent::Removed {
                            file: entry.file.clone(),
                        });
                    }
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                    Err(source) => return Err(AssetError::ManifestIo { path, source }.into()),
                }
            }
        }

        let manifest = Manifest {
            version: MANIFEST_VERSION,
            assets: entries,
        };
        manifest
            .save(&manifest_path)
            .map_err(|source| AssetError::ManifestIo {
                path: manifest_path,
                source,
            })?;

        self.events.emit(&BundleEvent::Finished {
            bundled,
            unchanged,
            removed,
        });

        Ok(())
    }

    /// Run [`Bundler::prepare`] over every asset, returning the results in
    /// declaration order.
    ///
    /// Workers pull the next asset off a shared cursor as they go free, so a
    /// slow download only ever holds up its own worker. Each keeps its results
    /// local, tagged with the asset's position, and the tags put them back in
    /// order at the end.
    fn prepare_all(
        &self,
        assets: &[RawAsset],
        existing: &HashMap<Asset, ManifestEntry>,
        out_dir: &Path,
    ) -> Vec<Result<Prepared, BundleError>> {
        let workers = self.parallelism.min(assets.len());
        if workers <= 1 {
            return assets
                .iter()
                .map(|asset| self.prepare(asset, existing, out_dir))
                .collect();
        }

        let next = AtomicUsize::new(0);
        let mut results: Vec<_> = thread::scope(|scope| {
            let workers: Vec<_> = (0..workers)
                .map(|_| {
                    scope.spawn(|| {
                        let mut results = Vec::new();
                        loop {
                            let index = next.fetch_add(1, Ordering::Relaxed);
                            let Some(asset) = assets.get(index) else {
                                return results;
                            };
                            results.push((index, self.prepare(asset, existing, out_dir)));
                        }
                    })
                })
                .collect();

            workers
                .into_iter()
                .flat_map(|worker| worker.join().expect("bundler worker panicked"))
                .collect()
        });

        results.sort_unstable_by_key(|(index, _)| *index);
        results.into_iter().map(|(_, result)| result).collect()
    }

    /// Resolve one asset to its manifest entry, writing it into `out_dir`
    /// unless an identical copy is already there.
    fn prepare(
        &self,
        asset: &RawAsset,
        existing: &HashMap<Asset, ManifestEntry>,
        out_dir: &Path,
    ) -> Result<Prepared, BundleError> {
        let source = asset.source();
        let src = match &source {
            Source::Path(p) => p.clone(),
            Source::Url(uri) => self.cache.fetch(uri)?,
        };
        let bytes = fs::read(&src).map_err(|source| AssetError::AssetIo {
            asset: Box::new(asset.clone()),
            source,
        })?;
        let digest = Sha256::digest(&bytes);
        let mut hash = String::with_capacity(digest.len() * 2);
        for b in &digest {
            let _ = write!(hash, "{b:02x}");
        }

        if let Some(expected) = asset.options().checksum() {
            let expected_digest = expected.strip_prefix("sha256:").ok_or_else(|| {
                AssetError::UnsupportedChecksum {
                    asset: Box::new(asset.clone()),
                    checksum: expected.to_owned(),
                }
            })?;
            if expected_digest != hash {
                return Err(AssetError::ChecksumMismatch {
                    asset: Box::new(asset.clone()),
                    expected: expected.to_owned(),
                    actual: format!("sha256:{hash}"),
                }
                .into());
            }
        }

        let short_hash = &hash[..16];

        let name = source.display_name();
        let name_path = Path::new(&name);
        let derived_stem = name_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("asset");
        let derived_ext = name_path.extension().and_then(|e| e.to_str());
        let options = asset.options();
        let stem = options.rename().unwrap_or(derived_stem);
        let ext = options.extension().or(derived_ext);
        let file = match (stem.is_empty(), ext) {
            (true, Some(ext)) if !ext.is_empty() => format!("{short_hash}.{ext}"),
            (true, _) => short_hash.to_string(),
            (false, Some(ext)) if !ext.is_empty() => format!("{stem}-{short_hash}.{ext}"),
            (false, _) => format!("{stem}.{short_hash}"),
        };

        let content_type = options.content_type().map_or_else(
            || {
                mime_guess::from_path(&file)
                    .first_or_octet_stream()
                    .to_string()
            },
            str::to_owned,
        );

        let id = asset.id();
        let dst = out_dir.join(&file);
        let unchanged = existing
            .get(&id)
            .is_some_and(|prev| prev.hash == hash && prev.file == file);

        let written = !unchanged || !dst.exists();
        if written {
            fs::write(&dst, &bytes).map_err(|source| AssetError::AssetIo {
                asset: Box::new(asset.clone()),
                source,
            })?;
            self.events.emit(&BundleEvent::Bundled {
                id,
                file: file.clone(),
                bytes: bytes.len(),
            });
        } else {
            self.events.emit(&BundleEvent::Unchanged {
                id,
                file: file.clone(),
            });
        }

        Ok(Prepared {
            entry: ManifestEntry {
                id,
                file,
                hash,
                content_type,
            },
            written,
        })
    }
}

/// One asset's manifest entry, plus whether it had to be written.
struct Prepared {
    entry: ManifestEntry,
    written: bool,
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use crate::{AssetOptions, ENCODED_ASSET_SIZE};

    use super::*;

    /// A fresh directory under the system temp directory.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("topcoat-asset-bundler-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sha256(contents: &str) -> String {
        let digest = Sha256::digest(contents.as_bytes());
        let mut hash = String::with_capacity(digest.len() * 2);
        for b in &digest {
            let _ = write!(hash, "{b:02x}");
        }
        hash
    }

    /// A scratch project: source files on disk, plus the encoded declarations a
    /// compiled binary would carry for them.
    struct Fixture {
        root: PathBuf,
        binary: Vec<u8>,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            Self {
                root: temp_dir(name),
                binary: Vec::new(),
            }
        }

        fn out(&self) -> PathBuf {
            self.root.join("out")
        }

        /// Write `contents` to `name` and declare it as an asset.
        fn declare(&mut self, name: &str, contents: &str) -> Asset {
            self.declare_with(name, contents, &AssetOptions::NONE)
        }

        fn declare_with(&mut self, name: &str, contents: &str, options: &AssetOptions) -> Asset {
            let src = self.root.join("src");
            fs::create_dir_all(&src).unwrap();
            let path = src.join(name);
            fs::write(&path, contents).unwrap();
            self.declare_path(path.to_str().unwrap(), options)
        }

        /// Declare an asset without creating the file it points at.
        fn declare_missing(&mut self, name: &str) -> Asset {
            let path = self.root.join("src").join(name);
            let path = path.to_str().unwrap().to_owned();
            self.declare_path(&path, &AssetOptions::NONE)
        }

        fn declare_path(&mut self, path: &str, options: &AssetOptions) -> Asset {
            let id = Asset::new("test", "src/lib.rs", path, options);
            self.binary.extend_from_slice(&RawAsset::encode(
                id,
                path,
                "test",
                "/test",
                "src/lib.rs",
                options,
            ));
            id
        }

        /// Drop every declaration past the first `count`, as if the assets had
        /// been deleted from the source and the binary rebuilt.
        fn keep_declarations(&mut self, count: usize) {
            self.binary.truncate(count * ENCODED_ASSET_SIZE);
        }

        fn bundle(&self, out: &Path, parallelism: usize) -> (BundleResult, Vec<BundleEvent>) {
            let (config, events) = BundlerConfig::new()
                .cache_dir(self.root.join("cache"))
                .parallelism(parallelism)
                .event_channel();

            // Both the config and the bundler hold senders; the receiver only
            // disconnects once every one of them is dropped.
            let result = Bundler::new(&config).bundle(&self.binary, out);
            drop(config);

            (result, events.into_iter().collect())
        }

        fn manifest(out: &Path) -> Manifest {
            Manifest::load(out.join(MANIFEST_NAME)).unwrap()
        }
    }

    #[test]
    fn manifest_follows_declaration_order_under_parallelism() {
        let mut fixture = Fixture::new("declaration-order");
        for i in 0..32 {
            fixture.declare(&format!("file-{i}.txt"), &format!("contents {i}"));
        }

        let out = fixture.out();
        let (result, _) = fixture.bundle(&out, 8);
        result.unwrap();

        let manifest = Fixture::manifest(&out);
        assert_eq!(manifest.assets.len(), 32);
        for (i, entry) in manifest.assets.iter().enumerate() {
            assert!(
                entry.file.starts_with(&format!("file-{i}-")),
                "entry {i} is out of order: {}",
                entry.file
            );
            assert_eq!(
                fs::read_to_string(out.join(&entry.file)).unwrap(),
                format!("contents {i}")
            );
        }
    }

    #[test]
    fn parallelism_does_not_change_the_output() {
        let mut fixture = Fixture::new("parallelism-agnostic");
        for i in 0..16 {
            fixture.declare(&format!("file-{i}.txt"), &format!("contents {i}"));
        }

        let sequential = fixture.root.join("sequential");
        let parallel = fixture.root.join("parallel");
        fixture.bundle(&sequential, 1).0.unwrap();
        fixture.bundle(&parallel, 8).0.unwrap();

        let sequential = Fixture::manifest(&sequential);
        let parallel = Fixture::manifest(&parallel);
        assert_eq!(sequential.assets.len(), parallel.assets.len());
        for (a, b) in sequential.assets.iter().zip(&parallel.assets) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.file, b.file);
            assert_eq!(a.hash, b.hash);
            assert_eq!(a.content_type, b.content_type);
        }
    }

    #[test]
    fn more_workers_than_assets_is_harmless() {
        let mut fixture = Fixture::new("excess-workers");
        fixture.declare("only.txt", "just the one");

        let out = fixture.out();
        let (result, events) = fixture.bundle(&out, 64);
        result.unwrap();

        assert_eq!(Fixture::manifest(&out).assets.len(), 1);
        assert!(matches!(
            events.last(),
            Some(BundleEvent::Finished { bundled: 1, .. })
        ));
    }

    #[test]
    fn an_empty_binary_produces_an_empty_manifest() {
        let fixture = Fixture::new("empty");

        let out = fixture.out();
        let (result, events) = fixture.bundle(&out, 8);
        result.unwrap();

        assert!(Fixture::manifest(&out).assets.is_empty());
        assert!(matches!(
            events.first(),
            Some(BundleEvent::Scanned { count: 0 })
        ));
        assert!(matches!(
            events.last(),
            Some(BundleEvent::Finished {
                bundled: 0,
                unchanged: 0,
                removed: 0,
            })
        ));
    }

    #[test]
    fn a_second_run_leaves_everything_unchanged() {
        let mut fixture = Fixture::new("unchanged");
        for i in 0..8 {
            fixture.declare(&format!("file-{i}.txt"), &format!("contents {i}"));
        }

        let out = fixture.out();
        fixture.bundle(&out, 4).0.unwrap();
        let (result, events) = fixture.bundle(&out, 4);
        result.unwrap();

        assert!(matches!(
            events.last(),
            Some(BundleEvent::Finished {
                bundled: 0,
                unchanged: 8,
                removed: 0,
            })
        ));
    }

    #[test]
    fn a_deleted_output_file_is_written_again() {
        let mut fixture = Fixture::new("restore-deleted");
        fixture.declare("kept.txt", "kept");
        fixture.declare("deleted.txt", "deleted");

        let out = fixture.out();
        fixture.bundle(&out, 2).0.unwrap();

        let deleted = Fixture::manifest(&out)
            .assets
            .into_iter()
            .find(|entry| entry.file.starts_with("deleted-"))
            .unwrap();
        fs::remove_file(out.join(&deleted.file)).unwrap();

        let (result, events) = fixture.bundle(&out, 2);
        result.unwrap();

        assert!(out.join(&deleted.file).exists());
        assert!(matches!(
            events.last(),
            Some(BundleEvent::Finished {
                bundled: 1,
                unchanged: 1,
                removed: 0,
            })
        ));
    }

    #[test]
    fn undeclared_files_are_removed() {
        let mut fixture = Fixture::new("remove-stale");
        fixture.declare("kept.txt", "kept");
        fixture.declare("stale.txt", "stale");

        let out = fixture.out();
        fixture.bundle(&out, 2).0.unwrap();
        let stale = Fixture::manifest(&out)
            .assets
            .into_iter()
            .find(|entry| entry.file.starts_with("stale-"))
            .unwrap();

        fixture.keep_declarations(1);
        let (result, events) = fixture.bundle(&out, 2);
        result.unwrap();

        assert!(!out.join(&stale.file).exists());
        assert_eq!(Fixture::manifest(&out).assets.len(), 1);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, BundleEvent::Removed { file } if *file == stale.file))
        );
        assert!(matches!(
            events.last(),
            Some(BundleEvent::Finished { removed: 1, .. })
        ));
    }

    #[test]
    fn a_matching_checksum_is_accepted() {
        let mut fixture = Fixture::new("checksum-match");
        let contents = "verified contents";
        fixture.declare_with(
            "verified.txt",
            contents,
            &AssetOptions {
                checksum: Some(format!("sha256:{}", sha256(contents)).into()),
                ..AssetOptions::NONE
            },
        );

        let out = fixture.out();
        fixture.bundle(&out, 1).0.unwrap();
        assert_eq!(Fixture::manifest(&out).assets.len(), 1);
    }

    #[test]
    fn a_mismatched_checksum_is_rejected() {
        let mut fixture = Fixture::new("checksum-mismatch");
        fixture.declare_with(
            "tampered.txt",
            "actual contents",
            &AssetOptions {
                checksum: Some(format!("sha256:{}", sha256("expected contents")).into()),
                ..AssetOptions::NONE
            },
        );

        let out = fixture.out();
        let error = fixture.bundle(&out, 1).0.unwrap_err();
        assert!(
            matches!(
                error,
                BundleError::Asset(AssetError::ChecksumMismatch { .. })
            ),
            "expected a checksum mismatch, got {error:?}"
        );
    }

    #[test]
    fn an_unsupported_checksum_algorithm_is_rejected() {
        let mut fixture = Fixture::new("checksum-algorithm");
        fixture.declare_with(
            "asset.txt",
            "contents",
            &AssetOptions {
                checksum: Some("md5:d41d8cd98f00b204e9800998ecf8427e".into()),
                ..AssetOptions::NONE
            },
        );

        let out = fixture.out();
        let error = fixture.bundle(&out, 1).0.unwrap_err();
        assert!(
            matches!(
                error,
                BundleError::Asset(AssetError::UnsupportedChecksum { .. })
            ),
            "expected an unsupported checksum, got {error:?}"
        );
    }

    #[test]
    fn rename_and_extension_shape_the_output_filename() {
        let mut fixture = Fixture::new("filename-options");
        fixture.declare_with(
            "source.txt",
            "renamed",
            &AssetOptions {
                rename: Some("styles".into()),
                extension: Some("css".into()),
                ..AssetOptions::NONE
            },
        );
        fixture.declare_with(
            "hashed.txt",
            "no stem",
            &AssetOptions {
                rename: Some("".into()),
                ..AssetOptions::NONE
            },
        );

        let out = fixture.out();
        fixture.bundle(&out, 2).0.unwrap();

        let manifest = Fixture::manifest(&out);
        let renamed = &manifest.assets[0];
        assert_eq!(renamed.file, format!("styles-{}.css", &renamed.hash[..16]));
        assert_eq!(renamed.content_type, "text/css");

        let hashed = &manifest.assets[1];
        assert_eq!(hashed.file, format!("{}.txt", &hashed.hash[..16]));
    }

    #[test]
    fn content_type_is_guessed_unless_overridden() {
        let mut fixture = Fixture::new("content-type");
        fixture.declare("guessed.css", "body {}");
        fixture.declare_with(
            "overridden.txt",
            "contents",
            &AssetOptions {
                content_type: Some("application/x-custom".into()),
                ..AssetOptions::NONE
            },
        );

        let out = fixture.out();
        fixture.bundle(&out, 2).0.unwrap();

        let manifest = Fixture::manifest(&out);
        assert_eq!(manifest.assets[0].content_type, "text/css");
        assert_eq!(manifest.assets[1].content_type, "application/x-custom");
    }

    #[test]
    fn the_earliest_declared_failure_is_reported() {
        let mut fixture = Fixture::new("earliest-failure");
        fixture.declare("present.txt", "fine");
        fixture.declare_missing("missing-first.txt");
        fixture.declare_missing("missing-second.txt");

        let out = fixture.out();
        let error = fixture.bundle(&out, 4).0.unwrap_err();
        match error {
            BundleError::Asset(AssetError::AssetIo { asset, .. }) => assert!(
                asset.source().to_string().ends_with("missing-first.txt"),
                "reported the wrong asset: {}",
                asset.source()
            ),
            other => panic!("expected an asset io error, got {other:?}"),
        }
    }

    #[test]
    fn a_failed_run_leaves_the_manifest_alone() {
        let mut fixture = Fixture::new("failure-keeps-manifest");
        fixture.declare("present.txt", "fine");

        let out = fixture.out();
        fixture.bundle(&out, 1).0.unwrap();
        let before = Fixture::manifest(&out).assets.len();

        fixture.declare_missing("missing.txt");
        fixture.bundle(&out, 2).0.unwrap_err();

        assert_eq!(Fixture::manifest(&out).assets.len(), before);
    }

    #[test]
    fn events_open_with_a_scan_and_close_with_a_summary() {
        let mut fixture = Fixture::new("event-order");
        for i in 0..4 {
            fixture.declare(&format!("file-{i}.txt"), &format!("contents {i}"));
        }

        let out = fixture.out();
        let (result, events) = fixture.bundle(&out, 2);
        result.unwrap();

        assert!(matches!(
            events.first(),
            Some(BundleEvent::Scanned { count: 4 })
        ));
        assert!(matches!(
            events.last(),
            Some(BundleEvent::Finished {
                bundled: 4,
                unchanged: 0,
                removed: 0,
            })
        ));
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, BundleEvent::Bundled { .. }))
                .count(),
            4
        );
    }
}
