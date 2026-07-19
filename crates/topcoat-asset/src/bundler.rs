mod cache;
mod dns;
mod error;

use std::{
    collections::{HashMap, HashSet},
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use http::Uri;
use sha2::{Digest, Sha256};

use crate::{
    AssetError, MANIFEST_NAME, MANIFEST_VERSION, Manifest, ManifestEntry, RawAsset, Source,
};

use self::cache::Cache;
use self::dns::CachingResolver;
pub use self::error::{BundleError, BundleResult};

/// How many remote assets a [`Bundler`] downloads concurrently by default.
///
/// Override with [`Bundler::parallelism`].
pub const DEFAULT_PARALLELISM: usize = 8;

/// The end-to-end timeout the default agent applies to each download.
///
/// Construct the bundler with [`Bundler::with_agent`] to use a different
/// timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_mins(1);

type FetchCallback = Box<dyn Fn(FetchEvent<'_>) + Send + Sync>;

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
    on_fetch: Option<FetchCallback>,
}

impl Bundler {
    /// Create a bundler with a default [`ureq::Agent`] configured with this
    /// crate's user agent, a [`DEFAULT_TIMEOUT`] per download, and DNS
    /// lookups cached per host for the life of the bundler.
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        let config = ureq::Agent::config_builder()
            .user_agent(concat!("topcoat-asset/", env!("CARGO_PKG_VERSION")))
            .timeout_global(Some(DEFAULT_TIMEOUT))
            .build();
        let agent = ureq::Agent::with_parts(
            config,
            ureq::unversioned::transport::DefaultConnector::default(),
            CachingResolver::default(),
        );
        Self::with_agent(cache_dir, agent)
    }

    /// Like [`Bundler::new`], but with a caller-supplied [`ureq::Agent`]
    /// (for custom timeouts, proxies, auth, etc.).
    pub fn with_agent(cache_dir: impl Into<PathBuf>, agent: ureq::Agent) -> Self {
        Self {
            cache: Cache::new(cache_dir.into(), agent),
            parallelism: DEFAULT_PARALLELISM,
            on_fetch: None,
        }
    }

    /// Set how many remote assets are downloaded concurrently.
    ///
    /// Values are clamped to at least 1. Defaults to [`DEFAULT_PARALLELISM`].
    #[must_use]
    pub fn parallelism(mut self, parallelism: usize) -> Self {
        self.parallelism = parallelism.max(1);
        self
    }

    /// Register a callback observing remote fetches during [`Bundler::bundle`].
    ///
    /// The bundler itself never prints; use this to surface download progress
    /// (see [`FetchEvent`]). The callback is invoked once per distinct remote
    /// URI, from whichever worker thread handled it, so it may be called from
    /// several threads at once.
    #[must_use]
    pub fn on_fetch(mut self, on_fetch: impl Fn(FetchEvent<'_>) + Send + Sync + 'static) -> Self {
        self.on_fetch = Some(Box::new(on_fetch));
        self
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
        let downloads = self.fetch_remote(&assets)?;
        let mut entries = Vec::with_capacity(assets.len());
        let mut kept_files = HashSet::with_capacity(assets.len());

        for asset in assets {
            let source = asset.source();
            let src = match &source {
                Source::Path(p) => p.clone(),
                Source::Url(uri) => downloads[&uri.to_string()].clone(),
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

    /// Fetch every distinct remote URI among `assets` into the cache, at most
    /// [`Self::parallelism`] downloads at a time, and return the local path
    /// for each URI.
    ///
    /// Identical URIs are fetched once, so concurrent workers never touch the
    /// same cache entry. Every URI is attempted even when one fails, and the
    /// error returned is the one for the earliest failing URI in declaration
    /// order; neither output nor errors depend on completion order.
    fn fetch_remote(&self, assets: &[RawAsset]) -> Result<HashMap<String, PathBuf>, BundleError> {
        let mut uris = Vec::new();
        let mut seen = HashSet::new();
        for asset in assets {
            if let Source::Url(uri) = asset.source()
                && seen.insert(uri.to_string())
            {
                uris.push(uri);
            }
        }

        let results: Vec<_> = uris.iter().map(|_| Mutex::new(None)).collect();
        let next = AtomicUsize::new(0);
        let workers = self.parallelism.min(uris.len());
        thread::scope(|scope| {
            for _ in 0..workers {
                scope.spawn(|| {
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(uri) = uris.get(index) else { break };
                        let result = self.fetch(uri);
                        *results[index].lock().expect("fetch result lock poisoned") = Some(result);
                    }
                });
            }
        });

        let mut downloads = HashMap::with_capacity(uris.len());
        for (uri, result) in uris.iter().zip(results) {
            let result = result
                .into_inner()
                .expect("fetch result lock poisoned")
                .expect("every uri is claimed by a worker");
            downloads.insert(uri.to_string(), result?);
        }
        Ok(downloads)
    }

    /// Resolve one remote URI to a local file, downloading on a cache miss,
    /// and report the outcome to the [`Self::on_fetch`] callback.
    fn fetch(&self, uri: &Uri) -> Result<PathBuf, BundleError> {
        if let Some(path) = self.cache.lookup(uri) {
            self.emit(FetchEvent::CacheHit { uri });
            return Ok(path);
        }
        let start = Instant::now();
        let path = self.cache.download(uri)?;
        self.emit(FetchEvent::Downloaded {
            uri,
            elapsed: start.elapsed(),
        });
        Ok(path)
    }

    fn emit(&self, event: FetchEvent<'_>) {
        if let Some(on_fetch) = &self.on_fetch {
            on_fetch(event);
        }
    }
}

/// A progress notification passed to the callback registered with
/// [`Bundler::on_fetch`] as [`Bundler::bundle`] fetches remote assets.
#[derive(Debug)]
#[non_exhaustive]
pub enum FetchEvent<'a> {
    /// A remote asset was downloaded over the network.
    Downloaded {
        /// The asset's source URL.
        uri: &'a Uri,
        /// How long the download took.
        elapsed: Duration,
    },
    /// A remote asset was already in the bundler's cache directory; no
    /// request was made.
    CacheHit {
        /// The asset's source URL.
        uri: &'a Uri,
    },
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read as _, Write as _},
        net::{TcpListener, TcpStream},
        sync::{Arc, atomic::AtomicUsize},
    };

    use super::*;
    use crate::{Asset, AssetOptions, ENCODED_ASSET_SIZE};

    /// A minimal HTTP server that counts requests and delays responses by
    /// path, so tests can control download completion order.
    struct TestServer {
        addr: std::net::SocketAddr,
        requests: Arc<AtomicUsize>,
    }

    impl TestServer {
        fn spawn(delay_for: fn(&str) -> Duration) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
            let addr = listener.local_addr().expect("test server addr");
            let requests = Arc::new(AtomicUsize::new(0));
            let counter = Arc::clone(&requests);
            thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { break };
                    counter.fetch_add(1, Ordering::SeqCst);
                    thread::spawn(move || serve_one(stream, delay_for));
                }
            });
            Self { addr, requests }
        }

        fn url(&self, path: &str) -> String {
            format!("http://{}{path}", self.addr)
        }

        fn request_count(&self) -> usize {
            self.requests.load(Ordering::SeqCst)
        }
    }

    fn serve_one(mut stream: TcpStream, delay_for: fn(&str) -> Duration) {
        let mut head = Vec::new();
        let mut buf = [0u8; 1024];
        while !head.windows(4).any(|w| w == b"\r\n\r\n") {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => head.extend_from_slice(&buf[..n]),
            }
        }
        let head = String::from_utf8_lossy(&head);
        let path = head
            .split_whitespace()
            .nth(1)
            .unwrap_or_default()
            .to_owned();
        thread::sleep(delay_for(&path));
        let body = format!("content of {path}");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len(),
        );
        let _ = stream.write_all(response.as_bytes());
    }

    /// Encode an `asset!`-equivalent declaration; distinct `source_file`s
    /// yield distinct asset ids for the same path.
    fn encode_asset(path: &str, source_file: &str) -> [u8; ENCODED_ASSET_SIZE] {
        let options = AssetOptions::NONE;
        let id = Asset::new("test_crate", source_file, path, &options);
        RawAsset::encode(
            id,
            path,
            "test_crate",
            "/nonexistent",
            source_file,
            &options,
        )
    }

    fn binary_of(assets: &[[u8; ENCODED_ASSET_SIZE]]) -> Vec<u8> {
        let mut binary = b"junk".to_vec();
        for asset in assets {
            binary.extend_from_slice(asset);
            binary.extend_from_slice(b"padding");
        }
        binary
    }

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "topcoat-bundler-test-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn bundles_local_assets_without_network() {
        let dir = test_dir("local");
        let asset_path = dir.join("logo.css");
        fs::write(&asset_path, "body {}").expect("write asset");

        let binary = binary_of(&[encode_asset(
            asset_path.to_str().expect("utf-8 path"),
            "a.rs",
        )]);
        let out_dir = dir.join("out");
        Bundler::new(dir.join("cache"))
            .bundle(&binary, &out_dir)
            .expect("bundle");

        let manifest = Manifest::load(out_dir.join(MANIFEST_NAME)).expect("load manifest");
        assert_eq!(manifest.assets.len(), 1);
        let entry = &manifest.assets[0];
        assert!(entry.file.starts_with("logo-"));
        assert_eq!(Path::new(&entry.file).extension(), Some("css".as_ref()));
        let bundled = fs::read_to_string(out_dir.join(&entry.file)).expect("read bundled");
        assert_eq!(bundled, "body {}");
    }

    #[test]
    fn deduplicates_identical_uris_and_reports_fetches() {
        let server = TestServer::spawn(|_| Duration::ZERO);
        let dir = test_dir("dedup");
        let url = server.url("/shared.css");
        let binary = binary_of(&[encode_asset(&url, "a.rs"), encode_asset(&url, "b.rs")]);

        let events = Arc::new(Mutex::new(Vec::new()));
        let bundler = |events: &Arc<Mutex<Vec<String>>>| {
            let events = Arc::clone(events);
            Bundler::new(dir.join("cache")).on_fetch(move |event| {
                let name = match event {
                    FetchEvent::Downloaded { .. } => "downloaded",
                    FetchEvent::CacheHit { .. } => "cache hit",
                };
                events.lock().expect("events lock").push(name.to_owned());
            })
        };

        bundler(&events)
            .bundle(&binary, dir.join("out"))
            .expect("bundle");
        assert_eq!(server.request_count(), 1);
        assert_eq!(*events.lock().expect("events lock"), ["downloaded"]);

        let manifest = Manifest::load(dir.join("out").join(MANIFEST_NAME)).expect("load manifest");
        assert_eq!(manifest.assets.len(), 2);
        assert_eq!(manifest.assets[0].file, manifest.assets[1].file);

        // A second run is served entirely from the cache directory.
        events.lock().expect("events lock").clear();
        bundler(&events)
            .bundle(&binary, dir.join("out2"))
            .expect("bundle again");
        assert_eq!(server.request_count(), 1);
        assert_eq!(*events.lock().expect("events lock"), ["cache hit"]);
    }

    #[test]
    fn manifest_order_is_declaration_order() {
        // The first-declared asset finishes last: with four concurrent
        // downloads, completion order inverts declaration order.
        let server = TestServer::spawn(|path| {
            if path.starts_with("/0") {
                Duration::from_millis(200)
            } else {
                Duration::ZERO
            }
        });
        let dir = test_dir("order");
        let assets: Vec<_> = (0..4)
            .map(|i| encode_asset(&server.url(&format!("/{i}.css")), "a.rs"))
            .collect();
        let binary = binary_of(&assets);

        Bundler::new(dir.join("cache"))
            .parallelism(4)
            .bundle(&binary, dir.join("out"))
            .expect("bundle");

        let manifest = Manifest::load(dir.join("out").join(MANIFEST_NAME)).expect("load manifest");
        let stems: Vec<_> = manifest
            .assets
            .iter()
            .map(|entry| entry.file.split('-').next().expect("file stem"))
            .collect();
        assert_eq!(stems, ["0", "1", "2", "3"]);
        assert_eq!(server.request_count(), 4);
    }
}
