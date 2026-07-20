use std::{
    fmt::Write as _,
    fs, io,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use http::Uri;
use sha2::{Digest, Sha256};

use super::{BundleError, BundleEvent, BundleEvents, BundlerConfig};

pub struct Cache {
    dir: PathBuf,
    agent: ureq::Agent,
    events: BundleEvents,
}

impl Cache {
    pub fn new(config: &BundlerConfig) -> Self {
        Self {
            dir: config.resolve_cache_dir(),
            agent: config.resolve_agent(),
            events: config.events().clone(),
        }
    }

    /// Return the local path of `uri`'s cached contents, downloading first if needed.
    ///
    /// Performs a blocking HTTP request when the asset isn't already cached. Safe to
    /// call concurrently; two threads racing on the same `uri` each download it and
    /// then atomically replace the cache entry with identical contents.
    pub fn fetch(&self, uri: &Uri) -> Result<PathBuf, BundleError> {
        let path = self.cached_path(uri);
        if path.exists() {
            self.events.emit(&BundleEvent::CacheHit {
                uri: uri.clone(),
                path: path.clone(),
            });
            return Ok(path);
        }

        fs::create_dir_all(&self.dir).map_err(|source| BundleError::CacheIo {
            path: self.dir.clone(),
            source,
        })?;

        self.events
            .emit(&BundleEvent::DownloadStarted { uri: uri.clone() });

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

        // Stream to a private tempfile then rename so a partial download can't be
        // mistaken for a hit, and so concurrent downloads of the same uri can't
        // interleave into one another's file.
        let tmp = {
            static SEQUENCE: AtomicU64 = AtomicU64::new(0);

            let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
            path.with_extension(format!("download-{}-{sequence}", std::process::id()))
        };

        let mut file = fs::File::create(&tmp).map_err(|source| BundleError::CacheIo {
            path: tmp.clone(),
            source,
        })?;
        let bytes = match io::copy(&mut reader, &mut file) {
            Ok(bytes) => bytes,
            Err(source) => {
                drop(file);
                let _ = fs::remove_file(&tmp);
                return Err(BundleError::CacheIo { path: tmp, source });
            }
        };
        drop(file);
        fs::rename(&tmp, &path).map_err(|source| BundleError::CacheIo {
            path: path.clone(),
            source,
        })?;

        self.events.emit(&BundleEvent::Downloaded {
            uri: uri.clone(),
            path: path.clone(),
            bytes,
        });

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
