mod cache;
mod error;

use std::{
    collections::{HashMap, HashSet},
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::{
    AssetError, MANIFEST_NAME, MANIFEST_VERSION, Manifest, ManifestEntry, RawAsset, Source,
};

use self::cache::Cache;
pub use self::error::{BundleError, BundleResult};

/// Scans a built binary for [`asset!`](crate::asset) declarations and
/// writes the referenced files into a bundle directory.
///
/// Local paths are copied; remote URLs are downloaded into `cache_dir`
/// (and reused on subsequent runs). Output filenames include a short
/// content hash, and the resulting directory is described by a
/// [`Manifest`].
pub struct Bundler {
    cache: Cache,
}

impl Bundler {
    /// Create a bundler with a default [`ureq::Agent`] configured with
    /// this crate's user agent.
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        let agent = ureq::Agent::config_builder()
            .user_agent(concat!("topcoat-asset/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        Self::with_agent(cache_dir, agent)
    }

    /// Like [`Bundler::new`], but with a caller-supplied [`ureq::Agent`]
    /// (for custom timeouts, proxies, auth, etc.).
    pub fn with_agent(cache_dir: impl Into<PathBuf>, agent: ureq::Agent) -> Self {
        Self {
            cache: Cache::new(cache_dir.into(), agent),
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
    /// This blocks on filesystem and network I/O; call it from a blocking
    /// context (e.g. [`tokio::task::spawn_blocking`]) when running inside an
    /// async runtime.
    ///
    /// # Errors
    ///
    /// Returns a [`BundleError`] for any I/O failure while creating or
    /// reading `out_dir` and its manifest, downloading a remote asset, or
    /// reading/writing a bundled file; and for a checksum mismatch between a
    /// declared asset and its configured `checksum`.
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
        let mut entries = Vec::with_capacity(assets.len());
        let mut kept_files = HashSet::with_capacity(assets.len());

        for asset in assets {
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

            if let Some(expected) = asset.options().checksum()
                && expected != hash
            {
                return Err(AssetError::ChecksumMismatch {
                    asset: Box::new(asset.clone()),
                    expected: expected.to_owned(),
                    actual: hash,
                }
                .into());
            }

            let short_hash = &hash[..8];

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

            if !unchanged || !dst.exists() {
                fs::write(&dst, &bytes).map_err(|source| AssetError::AssetIo {
                    asset: Box::new(asset.clone()),
                    source,
                })?;
            }

            kept_files.insert(file.clone());
            entries.push(ManifestEntry {
                id,
                file,
                hash,
                content_type,
            });
        }

        for entry in existing.values() {
            if !kept_files.contains(&entry.file) {
                let path = out_dir.join(&entry.file);
                match fs::remove_file(&path) {
                    Ok(()) => {}
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

        Ok(())
    }
}
