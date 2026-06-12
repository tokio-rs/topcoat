use std::path::PathBuf;

use http::Uri;
use sha2::{Digest, Sha256};
use tokio::fs;

use super::error::BundleError;

pub struct Cache {
    dir: PathBuf,
    client: reqwest::Client,
}

impl Cache {
    pub fn new(dir: PathBuf, client: reqwest::Client) -> Self {
        Self { dir, client }
    }

    /// Return the local path of `uri`'s cached contents, downloading first if needed.
    pub async fn fetch(&self, uri: &Uri) -> Result<PathBuf, BundleError> {
        let path = self.cached_path(uri);
        let exists = fs::try_exists(&path)
            .await
            .map_err(|source| BundleError::CacheIo {
                path: path.clone(),
                source,
            })?;
        if exists {
            return Ok(path);
        }

        fs::create_dir_all(&self.dir)
            .await
            .map_err(|source| BundleError::CacheIo {
                path: self.dir.clone(),
                source,
            })?;

        let response = self
            .client
            .get(uri.to_string())
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|source| BundleError::Download {
                uri: uri.clone(),
                source,
            })?;
        let bytes = response
            .bytes()
            .await
            .map_err(|source| BundleError::Download {
                uri: uri.clone(),
                source,
            })?;

        // Write to a sibling tempfile then rename so a partial download can't be mistaken for a
        // hit.
        let tmp = path.with_extension("download");
        fs::write(&tmp, &bytes)
            .await
            .map_err(|source| BundleError::CacheIo {
                path: tmp.clone(),
                source,
            })?;
        fs::rename(&tmp, &path)
            .await
            .map_err(|source| BundleError::CacheIo {
                path: path.clone(),
                source,
            })?;

        Ok(path)
    }

    fn cached_path(&self, uri: &Uri) -> PathBuf {
        let digest = Sha256::digest(uri.to_string().as_bytes());
        let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
        let key = &hex[..32];
        let name = match uri_extension(uri) {
            Some(ext) => format!("{key}.{ext}"),
            None => key.to_string(),
        };
        self.dir.join(name)
    }
}

fn uri_extension(uri: &Uri) -> Option<&str> {
    let last = uri.path().rsplit('/').find(|s| !s.is_empty())?;
    let dot = last.rfind('.')?;
    let ext = &last[dot + 1..];
    (!ext.is_empty()).then_some(ext)
}
