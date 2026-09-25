use std::{
    borrow::Cow,
    io,
    path::{Component, Path as FsPath, PathBuf as FsPathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use bytes::Bytes;
use futures_util::TryStreamExt;
use http::{
    HeaderValue, Method, StatusCode,
    header::{CONTENT_LENGTH, CONTENT_TYPE, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED},
};
use http_body::Frame;
use http_body_util::StreamBody;
use httpdate::HttpDate;
use tokio::{fs::File, io::AsyncReadExt};
use tokio_util::io::ReaderStream;
use topcoat_core::{
    context::Cx,
    error::{Error, Result},
};

use crate::{
    Body, IntoPath, Methods, Path, Route, RouteFuture, RouteId, RouterBuilder,
    error::not_found,
    path_param_segments,
    request::{headers, method},
    response::Response,
};

/// The maximum initial buffer size for file streaming.
const CHUNK_SIZE: usize = 64 * 1024;

/// Unix timestamp for the start of year 10000, beyond the HTTP date range.
const HTTP_DATE_END_SECS: u64 = 253_402_300_800;

/// A [`Route`] that serves files from a directory on disk.
///
/// Use directory serving for files that need fixed URLs or are created at
/// runtime. For bundled application files, use `asset!` to get content-hashed URLs.
///
/// For simpler registration, use
/// [`serve_dir`](RouterBuilderDirectoryExt::serve_dir) on the router builder,
/// or [`public_dir`](RouterBuilderDirectoryExt::public_dir) to serve files
/// at the site root.
///
/// The path must end in a catch-all parameter. Its value is the file path
/// relative to the directory. For example, `/public/{*file}` with directory
/// `public` serves `/public/css/site.css` from `public/css/site.css`.
///
/// Supports `GET` and `HEAD` and streams files from disk without caching them
/// in memory. Responses include `Content-Length` and a `Content-Type` based
/// on the file extension. A valid modification time adds `Last-Modified`.
/// Conditional requests can return `304 Not Modified` without a body.
///
/// Missing files, unreadable files, directories, and paths containing `..`
/// return `404 Not Found`. Dotfiles are served. Symbolic links are followed,
/// including links to files outside the directory.
///
/// # Examples
///
/// ```rust
/// use topcoat::router::{DirectoryRoute, Router};
///
/// let router = Router::builder()
///     .route(DirectoryRoute::new("/public/{*file}", "public"))
///     .build();
/// ```
///
/// Use `/{*file}` to serve files directly under the site root, such as
/// `/logo.svg`. The catch-all does not match `/` itself.
///
/// ```rust
/// use topcoat::router::{DirectoryRoute, Router};
///
/// let router = Router::builder()
///     .route(DirectoryRoute::new("/{*file}", "public"))
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct DirectoryRoute {
    /// The route's unique id.
    id: RouteId,
    /// The route pattern, ending in a catch-all.
    path: Cow<'static, Path>,
    /// The catch-all parameter's name.
    param: Box<str>,
    /// The directory containing the files.
    dir: FsPathBuf,
}

impl DirectoryRoute {
    /// Serves files from `dir` at `path`.
    ///
    /// The final catch-all parameter selects the file within `dir`. Relative
    /// directories are resolved against the process's current working
    /// directory on each request.
    ///
    /// # Panics
    ///
    /// Panics if `path` is invalid or does not end in a catch-all parameter.
    #[track_caller]
    pub fn new(path: impl IntoPath, dir: impl Into<FsPathBuf>) -> Self {
        let path = path.into_path();
        let Some(param) = path
            .segments()
            .next_back()
            .and_then(|segment| segment.as_catch_all().copied())
        else {
            panic!("directory route path `{path}` must end in a catch-all like `{{*file}}`");
        };
        Self {
            id: RouteId::new(),
            param: Box::from(param),
            path,
            dir: dir.into(),
        }
    }

    /// Returns the directory this route serves files from.
    #[must_use]
    pub fn dir(&self) -> &FsPath {
        &self.dir
    }

    /// Builds a filesystem path from the catch-all's decoded segments.
    ///
    /// Each segment must be a single filename. Rejects empty segments, `.`
    /// and `..`, and encoded path separators. Symbolic links can still lead
    /// outside the directory.
    fn resolve(&self, cx: &Cx) -> Option<FsPathBuf> {
        let mut file = self.dir.clone();
        for segment in path_param_segments(cx, &self.param) {
            let mut components = FsPath::new(segment).components();
            match (components.next(), components.next()) {
                (Some(Component::Normal(name)), None) if name == segment => file.push(name),
                _ => return None,
            }
        }
        Some(file)
    }

    /// Serves the requested file.
    async fn serve(&self, cx: &Cx) -> Result<Response> {
        let Some(path) = self.resolve(cx) else {
            return Err(not_found().into());
        };
        // Check the file type before opening. Opening a named pipe can block
        // indefinitely while waiting for a writer.
        if !tokio::fs::metadata(&path)
            .await
            .map_err(io_error)?
            .is_file()
        {
            return Err(not_found().into());
        }
        // Read metadata from the open handle. The file may have been replaced
        // since the check above.
        let file = File::open(&path).await.map_err(io_error)?;
        let metadata = file.metadata().await?;
        if !metadata.is_file() {
            return Err(not_found().into());
        }

        let mut response = Response::new(());
        let headers = response.headers_mut();
        let last_modified = metadata.modified().ok().and_then(http_date);
        if let Some(last_modified) = last_modified {
            headers.insert(
                LAST_MODIFIED,
                HeaderValue::try_from(last_modified.to_string())?,
            );
        }
        if is_not_modified(cx, last_modified) {
            *response.status_mut() = StatusCode::NOT_MODIFIED;
            return Ok(response.map(|()| Body::empty()));
        }
        let len = metadata.len();
        headers.insert(CONTENT_TYPE, content_type(&path));
        headers.insert(CONTENT_LENGTH, HeaderValue::from(len));
        let body = if method(cx) == Method::HEAD || len == 0 {
            Body::empty()
        } else {
            stream(file, len)
        };
        Ok(response.map(|()| body))
    }
}

impl Route for DirectoryRoute {
    fn id(&self) -> RouteId {
        self.id
    }

    fn methods(&self) -> Methods<'_> {
        Methods::Only(&[Method::GET])
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, _body: Body) -> RouteFuture<'cx> {
        Box::pin(self.serve(cx))
    }
}

/// Adds directory-serving methods to [`RouterBuilder`].
///
/// Use these methods for files that need fixed URLs or are created at runtime.
/// Use `asset!` for bundled files with content-hashed URLs.
///
/// Import this trait to use its methods. See [`DirectoryRoute`] for file
/// handling and path restrictions.
pub trait RouterBuilderDirectoryExt {
    /// Serves files from `dir` at `path`.
    ///
    /// The final catch-all parameter selects the file within `dir`. For
    /// example, the route below serves `/downloads/report.pdf` from
    /// `var/downloads/report.pdf`.
    ///
    /// Registers `DirectoryRoute::new(path, dir)` with the builder. See
    /// [`DirectoryRoute`] for file-serving behavior and restrictions.
    ///
    /// # Panics
    ///
    /// Panics if `path` is invalid or does not end in a catch-all parameter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use topcoat::router::{Router, RouterBuilderDirectoryExt};
    ///
    /// let router = Router::builder()
    ///     .serve_dir("/downloads/{*file}", "var/downloads")
    ///     .build();
    /// ```
    #[must_use]
    fn serve_dir(self, path: impl IntoPath, dir: impl Into<FsPathBuf>) -> Self;

    /// Serves files from `dir` at the site root.
    ///
    /// For example, `public_dir("./public")` serves `./public/logo.svg` at
    /// `/logo.svg`. This is shorthand for `serve_dir("/{*file}", dir)`.
    /// The catch-all does not match `/`, so a home page can be registered
    /// separately.
    ///
    /// Registers a [`DirectoryRoute`] with the same behavior and restrictions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use topcoat::router::{Router, RouterBuilderDirectoryExt};
    ///
    /// let router = Router::builder().public_dir("./public").build();
    /// ```
    #[must_use]
    fn public_dir(self, dir: impl Into<FsPathBuf>) -> Self;
}

impl RouterBuilderDirectoryExt for RouterBuilder {
    #[track_caller]
    fn serve_dir(self, path: impl IntoPath, dir: impl Into<FsPathBuf>) -> Self {
        self.route(DirectoryRoute::new(path, dir))
    }

    fn public_dir(self, dir: impl Into<FsPathBuf>) -> Self {
        self.serve_dir("/{*file}", dir)
    }
}

/// Maps file access errors to router errors.
///
/// Missing files, invalid paths, and permission errors become `404 Not Found`.
/// Other I/O errors are propagated.
fn io_error(error: io::Error) -> Error {
    match error.kind() {
        io::ErrorKind::NotFound
        | io::ErrorKind::NotADirectory
        | io::ErrorKind::PermissionDenied
        | io::ErrorKind::InvalidInput => not_found().into(),
        _ => error.into(),
    }
}

/// Converts a modification time to an HTTP date.
///
/// Clamps future timestamps to now. Returns `None` if the resulting date is
/// outside the range supported by `HttpDate`.
fn http_date(modified: SystemTime) -> Option<HttpDate> {
    let modified = modified.min(SystemTime::now());
    let secs = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();
    (secs < HTTP_DATE_END_SECS).then(|| HttpDate::from(modified))
}

/// Checks whether to return `304 Not Modified` for an existing file.
///
/// `If-None-Match` takes precedence. Only `*` matches because this route does
/// not generate `ETags`. When that header is absent, checks whether the file was
/// last modified at or before the date in `If-Modified-Since`. Missing or
/// invalid dates return `false`.
fn is_not_modified(cx: &Cx, last_modified: Option<HttpDate>) -> bool {
    let headers = headers(cx);
    match (headers.get(IF_NONE_MATCH), last_modified) {
        (Some(value), _) => value.as_bytes() == b"*",
        (None, Some(last_modified)) => headers
            .get(IF_MODIFIED_SINCE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<HttpDate>().ok())
            .is_some_and(|since| last_modified <= since),
        (None, None) => false,
    }
}

/// Returns the content type for the file extension, or `application/octet-stream`.
fn content_type(path: &FsPath) -> HeaderValue {
    // The MIME table contains static strings that are valid header values.
    HeaderValue::from_static(
        mime_guess::from_path(path)
            .first_raw()
            .unwrap_or("application/octet-stream"),
    )
}

/// Streams up to `len` bytes from `file`.
///
/// Limits reads to the advertised `Content-Length` if the file grows during
/// the response. Small files use a smaller initial buffer.
fn stream(file: File, len: u64) -> Body {
    let capacity = usize::try_from(len).map_or(CHUNK_SIZE, |len| len.min(CHUNK_SIZE));
    let chunks = ReaderStream::with_capacity(file.take(len), capacity).map_ok(Frame::<Bytes>::data);
    Body::new(StreamBody::new(chunks))
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        time::{Duration, SystemTime},
    };

    use http::{HeaderMap, Request, header::ACCEPT};
    use http_body_util::BodyExt;

    use super::*;
    use crate::Router;

    /// Creates an empty test directory, removing any files from a previous run.
    fn temp_dir(name: &str) -> FsPathBuf {
        let dir = env::temp_dir().join(format!("topcoat-router-directory-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Creates sample files in `dir`, including a nested stylesheet.
    fn populate(dir: &FsPath) {
        fs::write(dir.join("index.html"), "<h1>Hello</h1>").unwrap();
        fs::write(dir.join("logo.svg"), "<svg/>").unwrap();
        fs::write(dir.join("notes"), "no extension").unwrap();
        fs::create_dir_all(dir.join("css")).unwrap();
        fs::write(dir.join("css/site.css"), "body{}").unwrap();
    }

    fn router(path: &'static str, dir: &FsPath) -> Router {
        Router::builder()
            .route(DirectoryRoute::new(path, dir))
            .build()
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    /// Sends a request through the router and collects the response body.
    fn send(
        router: &Router,
        method: Method,
        path: &str,
        request_headers: &[(http::HeaderName, &str)],
    ) -> (StatusCode, HeaderMap, Bytes) {
        let mut request = Request::builder().method(method).uri(path);
        for (name, value) in request_headers {
            request = request.header(name, *value);
        }
        let request = request.body(Body::empty()).unwrap();
        let response = block_on(router.handle(request));
        let (parts, body) = response.into_parts();
        let bytes = block_on(body.collect()).unwrap().to_bytes();
        (parts.status, parts.headers, bytes)
    }

    fn get(router: &Router, path: &str) -> (StatusCode, HeaderMap, Bytes) {
        send(router, Method::GET, path, &[])
    }

    // -- Construction --

    #[test]
    fn keeps_the_path_and_directory() {
        let route = DirectoryRoute::new("/public/{*file}", "public");
        assert_eq!(route.path().as_str(), "/public/{*file}");
        assert_eq!(route.dir(), FsPath::new("public"));
    }

    #[test]
    fn catch_all_may_have_any_name() {
        let dir = temp_dir("param-name");
        populate(&dir);
        let router = router("/public/{*rest}", &dir);
        let (status, _, body) = get(&router, "/public/css/site.css");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "body{}");
    }

    #[test]
    #[should_panic(expected = "must end in a catch-all")]
    fn path_without_a_catch_all_panics() {
        let _ = DirectoryRoute::new("/public", "public");
    }

    #[test]
    #[should_panic(expected = "must end in a catch-all")]
    fn path_with_a_single_segment_param_panics() {
        let _ = DirectoryRoute::new("/public/{file}", "public");
    }

    #[test]
    fn only_get_and_its_head_alias_are_served() {
        let dir = temp_dir("methods");
        populate(&dir);
        let router = router("/public/{*file}", &dir);
        let (status, _, _) = send(&router, Method::POST, "/public/index.html", &[]);
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    // -- Serving --

    #[test]
    fn serves_a_file_with_its_type_and_length() {
        let dir = temp_dir("serves");
        populate(&dir);
        let router = router("/public/{*file}", &dir);

        let (status, headers, body) = get(&router, "/public/index.html");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[CONTENT_TYPE], "text/html");
        assert_eq!(headers[CONTENT_LENGTH], "14");
        assert!(headers.contains_key(LAST_MODIFIED));
        assert_eq!(body, "<h1>Hello</h1>");

        let (_, headers, body) = get(&router, "/public/logo.svg");
        assert_eq!(headers[CONTENT_TYPE], "image/svg+xml");
        assert_eq!(body, "<svg/>");
    }

    #[test]
    fn serves_nested_files() {
        let dir = temp_dir("nested");
        populate(&dir);
        let router = router("/public/{*file}", &dir);
        let (status, headers, body) = get(&router, "/public/css/site.css");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[CONTENT_TYPE], "text/css");
        assert_eq!(body, "body{}");
    }

    #[test]
    fn unknown_extension_is_an_octet_stream() {
        let dir = temp_dir("octet-stream");
        populate(&dir);
        let router = router("/public/{*file}", &dir);
        let (status, headers, _) = get(&router, "/public/notes");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[CONTENT_TYPE], "application/octet-stream");
    }

    #[test]
    fn serves_an_empty_file() {
        let dir = temp_dir("empty-file");
        populate(&dir);
        fs::write(dir.join("empty.txt"), "").unwrap();
        let router = router("/public/{*file}", &dir);
        let (status, headers, body) = get(&router, "/public/empty.txt");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[CONTENT_LENGTH], "0");
        assert!(body.is_empty());
    }

    #[test]
    fn serves_from_the_root_prefix() {
        let dir = temp_dir("root");
        populate(&dir);
        let router = router("/{*file}", &dir);
        let (status, _, body) = get(&router, "/css/site.css");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "body{}");
    }

    // -- RouterBuilderDirectoryExt --

    #[test]
    fn serve_dir_registers_the_route() {
        let dir = temp_dir("serve-dir");
        populate(&dir);
        let router = Router::builder().serve_dir("/public/{*file}", &dir).build();
        let (status, _, body) = get(&router, "/public/logo.svg");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "<svg/>");
    }

    #[test]
    fn public_dir_serves_at_the_root() {
        let dir = temp_dir("public-dir");
        populate(&dir);
        let router = Router::builder().public_dir(&dir).build();
        let (status, _, body) = get(&router, "/logo.svg");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "<svg/>");
        let (status, _, _) = get(&router, "/");
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[test]
    #[should_panic(expected = "must end in a catch-all")]
    fn serve_dir_without_a_catch_all_panics() {
        let _ = Router::builder().serve_dir("/public", "public");
    }

    #[test]
    fn head_sends_the_headers_without_the_body() {
        let dir = temp_dir("head");
        populate(&dir);
        let router = router("/public/{*file}", &dir);
        let (status, headers, body) = send(&router, Method::HEAD, "/public/index.html", &[]);
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[CONTENT_TYPE], "text/html");
        assert_eq!(headers[CONTENT_LENGTH], "14");
        assert!(body.is_empty());
    }

    // -- Not found --

    #[test]
    fn missing_file_is_not_found() {
        let dir = temp_dir("missing");
        populate(&dir);
        let router = router("/public/{*file}", &dir);
        let (status, _, _) = get(&router, "/public/missing.html");
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn directory_is_not_found() {
        let dir = temp_dir("directory");
        populate(&dir);
        let router = router("/public/{*file}", &dir);
        let (status, _, _) = get(&router, "/public/css");
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _, _) = get(&router, "/public/css/");
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn path_through_a_file_is_not_found() {
        let dir = temp_dir("through-file");
        populate(&dir);
        let router = router("/public/{*file}", &dir);
        let (status, _, _) = get(&router, "/public/index.html/more");
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn missing_directory_is_not_found() {
        let dir = temp_dir("missing-directory");
        let router = router("/public/{*file}", &dir.join("nowhere"));
        let (status, _, _) = get(&router, "/public/index.html");
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn parent_segments_cannot_escape_the_directory() {
        let root = temp_dir("escape");
        fs::write(root.join("secret.txt"), "secret").unwrap();
        let dir = root.join("public");
        fs::create_dir_all(&dir).unwrap();
        populate(&dir);
        let router = router("/public/{*file}", &dir);

        for path in [
            "/public/../secret.txt",
            "/public/%2E%2E/secret.txt",
            "/public/..%2Fsecret.txt",
            "/public/css/../../secret.txt",
        ] {
            let (status, _, _) = get(&router, path);
            assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        }
    }

    #[test]
    fn encoded_separators_stay_inside_their_segment() {
        let dir = temp_dir("encoded-separator");
        populate(&dir);
        let router = router("/public/{*file}", &dir);
        let (status, _, _) = get(&router, "/public/css%2Fsite.css");
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn empty_and_dot_segments_are_not_found() {
        let dir = temp_dir("dot-segments");
        populate(&dir);
        let router = router("/public/{*file}", &dir);
        for path in ["/public//index.html", "/public/./index.html"] {
            let (status, _, _) = get(&router, path);
            assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        }
    }

    // -- Conditional requests --

    #[test]
    fn unchanged_file_is_not_modified() {
        let dir = temp_dir("not-modified");
        populate(&dir);
        let router = router("/public/{*file}", &dir);

        let (_, headers, _) = get(&router, "/public/index.html");
        let last_modified = headers[LAST_MODIFIED].to_str().unwrap();
        let (status, headers, body) = send(
            &router,
            Method::GET,
            "/public/index.html",
            &[(IF_MODIFIED_SINCE, last_modified)],
        );
        assert_eq!(status, StatusCode::NOT_MODIFIED);
        assert_eq!(headers[LAST_MODIFIED], last_modified);
        assert!(!headers.contains_key(CONTENT_TYPE));
        assert!(!headers.contains_key(CONTENT_LENGTH));
        assert!(body.is_empty());
    }

    #[test]
    fn changed_file_is_sent_again() {
        let dir = temp_dir("modified");
        populate(&dir);
        let router = router("/public/{*file}", &dir);

        let modified = fs::metadata(dir.join("index.html"))
            .unwrap()
            .modified()
            .unwrap();
        let since = HttpDate::from(modified - Duration::from_secs(60)).to_string();
        let (status, _, body) = send(
            &router,
            Method::GET,
            "/public/index.html",
            &[(IF_MODIFIED_SINCE, &since)],
        );
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "<h1>Hello</h1>");
    }

    #[test]
    fn malformed_if_modified_since_sends_the_file() {
        let dir = temp_dir("malformed-since");
        populate(&dir);
        let router = router("/public/{*file}", &dir);
        let (status, _, body) = send(
            &router,
            Method::GET,
            "/public/index.html",
            &[(IF_MODIFIED_SINCE, "yesterday"), (ACCEPT, "*/*")],
        );
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "<h1>Hello</h1>");
    }

    #[test]
    fn if_none_match_star_is_not_modified() {
        let dir = temp_dir("if-none-match-star");
        populate(&dir);
        let router = router("/public/{*file}", &dir);
        for method in [Method::GET, Method::HEAD] {
            let (status, headers, body) = send(
                &router,
                method,
                "/public/index.html",
                &[(IF_NONE_MATCH, "*")],
            );
            assert_eq!(status, StatusCode::NOT_MODIFIED);
            assert!(headers.contains_key(LAST_MODIFIED));
            assert!(!headers.contains_key(CONTENT_TYPE));
            assert!(body.is_empty());
        }
        let (status, _, _) = send(
            &router,
            Method::GET,
            "/public/missing.html",
            &[(IF_NONE_MATCH, "*")],
        );
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn if_none_match_star_needs_no_last_modified() {
        let dir = temp_dir("if-none-match-star-ancient");
        populate(&dir);
        set_modified(
            &dir.join("index.html"),
            UNIX_EPOCH - Duration::from_hours(24),
        );
        let router = router("/public/{*file}", &dir);
        let (status, headers, _) = send(
            &router,
            Method::GET,
            "/public/index.html",
            &[(IF_NONE_MATCH, "*")],
        );
        assert_eq!(status, StatusCode::NOT_MODIFIED);
        assert!(!headers.contains_key(LAST_MODIFIED));
    }

    #[test]
    fn if_none_match_takes_precedence_over_if_modified_since() {
        let dir = temp_dir("if-none-match");
        populate(&dir);
        let router = router("/public/{*file}", &dir);

        let (_, headers, _) = get(&router, "/public/index.html");
        let last_modified = headers[LAST_MODIFIED].to_str().unwrap();
        let (status, _, body) = send(
            &router,
            Method::GET,
            "/public/index.html",
            &[
                (IF_MODIFIED_SINCE, last_modified),
                (IF_NONE_MATCH, "\"abc\""),
            ],
        );
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "<h1>Hello</h1>");
    }

    // -- Last-Modified --

    /// Sets the modification time of the file at `path`.
    fn set_modified(path: &FsPath, time: SystemTime) {
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(time)
            .unwrap();
    }

    #[test]
    fn last_modified_reflects_the_file() {
        let dir = temp_dir("last-modified");
        populate(&dir);
        let router = router("/public/{*file}", &dir);
        let modified = fs::metadata(dir.join("index.html"))
            .unwrap()
            .modified()
            .unwrap();
        let (_, headers, _) = get(&router, "/public/index.html");
        let sent: HttpDate = headers[LAST_MODIFIED].to_str().unwrap().parse().unwrap();
        assert_eq!(sent, HttpDate::from(modified));
    }

    #[test]
    fn future_modification_time_is_clamped_to_now() {
        let dir = temp_dir("future-modified");
        populate(&dir);
        set_modified(
            &dir.join("index.html"),
            SystemTime::now() + Duration::from_hours(24),
        );
        let router = router("/public/{*file}", &dir);
        let (status, headers, _) = get(&router, "/public/index.html");
        assert_eq!(status, StatusCode::OK);
        let sent: HttpDate = headers[LAST_MODIFIED].to_str().unwrap().parse().unwrap();
        assert!(SystemTime::from(sent) <= SystemTime::now());
    }

    #[test]
    fn modification_time_before_the_epoch_omits_the_header() {
        let dir = temp_dir("ancient-modified");
        populate(&dir);
        set_modified(
            &dir.join("index.html"),
            UNIX_EPOCH - Duration::from_hours(24),
        );
        let router = router("/public/{*file}", &dir);
        let (status, headers, body) = get(&router, "/public/index.html");
        assert_eq!(status, StatusCode::OK);
        assert!(!headers.contains_key(LAST_MODIFIED));
        assert_eq!(body, "<h1>Hello</h1>");
    }

    // -- Special files --

    #[cfg(unix)]
    #[test]
    fn named_pipe_is_not_found() {
        let dir = temp_dir("named-pipe");
        populate(&dir);
        let status = std::process::Command::new("mkfifo")
            .arg(dir.join("pipe"))
            .status()
            .unwrap();
        assert!(status.success());
        let router = router("/public/{*file}", &dir);
        // A blocked open would also prevent runtime shutdown. Use a timeout
        // and shut down without waiting so a regression fails instead of hanging.
        let request = Request::get("/public/pipe").body(Body::empty()).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let response = runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(5), router.handle(request)).await
        });
        runtime.shutdown_background();
        let response = response.expect("the request blocked on opening the pipe");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn body_is_cut_at_the_advertised_length() {
        let dir = temp_dir("cut-body");
        populate(&dir);
        let body = block_on(async {
            let file = File::open(dir.join("index.html")).await.unwrap();
            stream(file, 4).collect().await.unwrap().to_bytes()
        });
        assert_eq!(body, "<h1>");
    }
}
