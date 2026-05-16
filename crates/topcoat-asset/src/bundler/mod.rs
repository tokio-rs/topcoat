mod cache;
mod error;

use std::{
    collections::{HashMap, HashSet},
    io,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use tokio::fs;

use crate::{
    AssetError, MANIFEST_NAME, MANIFEST_VERSION, Manifest, ManifestEntry, RawAsset, Source,
};

use self::cache::Cache;
pub use self::error::{BundleError, BundleResult};

pub struct Bundler {
    cache: Cache,
}

impl Bundler {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!("topcoat-asset/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build default reqwest client");
        Self::with_client(cache_dir, client)
    }

    pub fn with_client(cache_dir: impl Into<PathBuf>, client: reqwest::Client) -> Self {
        Self {
            cache: Cache::new(cache_dir.into(), client),
        }
    }

    /// Scan `binary` for embedded assets and sync them into `out_dir`.
    ///
    /// If `out_dir` already contains a `manifest.toml`, it is loaded and used
    /// to skip copying files whose content hash hasn't changed. Files that
    /// were present in the old manifest but are no longer referenced by the
    /// new one are removed. Remote (http/https) assets are downloaded into
    /// the bundler's cache directory and then treated like local files.
    pub async fn bundle(&self, binary: &[u8], out_dir: impl AsRef<Path>) -> BundleResult {
        let out_dir = out_dir.as_ref();
        fs::create_dir_all(out_dir)
            .await
            .map_err(|source| AssetError::ManifestIo {
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
                Source::Url(uri) => self.cache.fetch(uri).await?,
            };
            let bytes = fs::read(&src).await.map_err(|source| AssetError::AssetIo {
                asset: asset.clone(),
                source,
            })?;
            let digest = Sha256::digest(&bytes);
            let hash = digest
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();
            let short_hash = &hash[..8];

            let name = source.display_name();
            let name_path = Path::new(&name);
            let stem = name_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("asset");
            let file = match name_path.extension().and_then(|e| e.to_str()) {
                Some(ext) => format!("{stem}-{short_hash}.{ext}"),
                None => format!("{stem}.{short_hash}"),
            };

            let id = asset.id();
            let dst = out_dir.join(&file);
            let unchanged = existing
                .get(&id)
                .is_some_and(|prev| prev.hash == hash && prev.file == file);

            if !unchanged || !dst.exists() {
                fs::write(&dst, &bytes)
                    .await
                    .map_err(|source| AssetError::AssetIo {
                        asset: asset.clone(),
                        source,
                    })?;
            }

            kept_files.insert(file.clone());
            entries.push(ManifestEntry { id, file, hash });
        }

        for entry in existing.values() {
            if !kept_files.contains(&entry.file) {
                let path = out_dir.join(&entry.file);
                match fs::remove_file(&path).await {
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
