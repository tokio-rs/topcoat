use std::{fmt::Write as _, fs, io, path::PathBuf};

use http::Uri;
use sha2::{Digest, Sha256};

use super::error::BundleError;

pub struct Cache {
    dir: PathBuf,
    agent: ureq::Agent,
}

impl Cache {
    pub fn new(dir: PathBuf, agent: ureq::Agent) -> Self {
        Self { dir, agent }
    }

    /// Return the local path of `uri`'s cached contents, if present.
    pub fn lookup(&self, uri: &Uri) -> Option<PathBuf> {
        let path = self.cached_path(uri);
        path.exists().then_some(path)
    }

    /// Download `uri` into the cache and return the local path of its contents.
    ///
    /// Performs a blocking HTTP request.
    pub fn download(&self, uri: &Uri) -> Result<PathBuf, BundleError> {
        let path = self.cached_path(uri);

        fs::create_dir_all(&self.dir).map_err(|source| BundleError::CacheIo {
            path: self.dir.clone(),
            source,
        })?;

        let mut body = self
            .agent
            .get(uri.to_string())
            .call()
            .map_err(|source| BundleError::Download {
                uri: uri.clone(),
                source: Box::new(source),
            })?
            .into_body();
        let mut reader = body.as_reader();

        // Stream to a sibling tempfile then rename so a partial download can't be mistaken for a
        // hit. The tempfile name includes the process id so another bundler process downloading
        // the same URI can't interleave writes with ours; whichever renames last wins atomically.
        let tmp = path.with_extension(format!("download.{}", std::process::id()));
        let mut file = fs::File::create(&tmp).map_err(|source| BundleError::CacheIo {
            path: tmp.clone(),
            source,
        })?;
        io::copy(&mut reader, &mut file).map_err(|source| BundleError::CacheIo {
            path: tmp.clone(),
            source,
        })?;
        drop(file);
        fs::rename(&tmp, &path).map_err(|source| BundleError::CacheIo {
            path: path.clone(),
            source,
        })?;

        Ok(path)
    }

    fn cached_path(&self, uri: &Uri) -> PathBuf {
        let digest = Sha256::digest(uri.to_string().as_bytes());
        let mut hex = String::with_capacity(digest.len() * 2);
        for b in &digest {
            let _ = write!(hex, "{b:02x}");
        }
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
