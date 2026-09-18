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
    Body, IntoPath, Methods, Path, Route, RouteFuture, RouteId,
    error::not_found,
    path_param_segments,
    request::{headers, method},
    response::Response,
};

/// The size of the chunks a file is streamed in.
const CHUNK_SIZE: usize = 64 * 1024;

/// The first second past the year 9999, the last year an HTTP date can name.
const HTTP_DATE_END_SECS: u64 = 253_402_300_800;

/// A [`Route`] that serves the files of a directory on disk.
///
/// The route's path ends in a catch-all, and what the catch-all captures is
/// the file's path inside the directory: a route at `/public/{*file}` serves
/// `/public/css/site.css` from `public/css/site.css`. A request that resolves
/// to nothing readable inside the directory, including one that tries to
/// escape it with `..`, responds with `404 Not Found`. Symbolic links are
/// followed.
///
/// Every file is sent with the `Content-Type` its extension suggests, its
/// `Content-Length`, and a `Last-Modified` header. A request carrying an
/// `If-Modified-Since` header for a file that has not changed since gets a
/// `304 Not Modified` response without a body. Files are not cached beyond
/// that, so a changed file is picked up on the next request.
///
/// Everything inside the directory is public, so point the route at a
/// directory that holds nothing else. Files that never change are better
/// declared with `asset!`, which hashes them into their URL so browsers can
/// cache them forever.
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
/// Registered at the root, the route serves any URL no other route claims:
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
    /// The identity of this route's handler.
    id: RouteId,
    /// The URL path this route handles, ending in a catch-all.
    path: Cow<'static, Path>,
    /// The name of the path's catch-all parameter.
    param: Box<str>,
    /// The directory the files are served from.
    dir: FsPathBuf,
}

impl DirectoryRoute {
    /// Creates a route serving the files of `dir` at `path`, whose catch-all
    /// captures the file to serve.
    ///
    /// A relative `dir` is resolved against the working directory of the
    /// process at request time.
    ///
    /// # Panics
    ///
    /// Panics if `path` is a string that is not a well-formed route path, or
    /// if it does not end in a catch-all.
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

    /// The directory the files are served from.
    #[must_use]
    pub fn dir(&self) -> &FsPath {
        &self.dir
    }

    /// Maps the requested URL onto a path inside the directory.
    ///
    /// Returns `None` when a segment is not a plain file name: an empty
    /// segment, `.` or `..`, or one holding a separator that was encoded in
    /// the URL. Refusing those means a request cannot name anything outside
    /// the directory, though a symbolic link inside it may still point out.
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

    /// Builds the response for the file the request resolves to.
    async fn serve(&self, cx: &Cx) -> Result<Response> {
        let Some(path) = self.resolve(cx) else {
            return Err(not_found().into());
        };
        // The file type is checked before the file is opened: a directory has
        // nothing to send, and opening a special file like a named pipe can
        // block until a peer shows up.
        let metadata = tokio::fs::metadata(&path).await.map_err(io_error)?;
        if !metadata.is_file() {
            return Err(not_found().into());
        }

        let last_modified = metadata.modified().ok().and_then(http_date);
        let mut response = Response::new(Body::empty());
        let headers = response.headers_mut();
        if let Some(last_modified) = last_modified {
            headers.insert(LAST_MODIFIED, last_modified.to_string().parse()?);
            if is_unchanged_since(cx, last_modified) {
                *response.status_mut() = StatusCode::NOT_MODIFIED;
                return Ok(response);
            }
        }
        let len = metadata.len();
        headers.insert(CONTENT_TYPE, content_type(&path));
        headers.insert(CONTENT_LENGTH, HeaderValue::from(len));
        if method(cx) != Method::HEAD {
            let file = File::open(&path).await.map_err(io_error)?;
            *response.body_mut() = stream(file, len);
        }
        Ok(response)
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

/// Converts a failure to reach a file into the error to respond with.
///
/// A failure meaning there is no file to serve at the path becomes
/// `404 Not Found`: besides a missing file that covers a path running through
/// a regular file, one the process may not read, and one the platform rejects
/// outright. Any other failure is reported as is.
fn io_error(error: io::Error) -> Error {
    match error.kind() {
        io::ErrorKind::NotFound
        | io::ErrorKind::NotADirectory
        | io::ErrorKind::PermissionDenied
        | io::ErrorKind::InvalidInput => not_found().into(),
        _ => error.into(),
    }
}

/// Converts a file's modification time into a `Last-Modified` value.
///
/// A time in the future is clamped to now, as a `Last-Modified` must not be
/// later than the response it is sent with. A time an HTTP date cannot
/// express, before 1970 or past the year 9999, yields `None` so the header is
/// left out.
fn http_date(modified: SystemTime) -> Option<HttpDate> {
    let modified = modified.min(SystemTime::now());
    let secs = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();
    (secs < HTTP_DATE_END_SECS).then(|| HttpDate::from(modified))
}

/// Returns whether the request's `If-Modified-Since` header names a time no
/// earlier than the file's `last_modified`.
///
/// A missing or malformed header never matches, so the file is sent. Neither
/// does one accompanied by `If-None-Match`, which takes precedence and, as
/// files carry no `ETag`, can never match either.
fn is_unchanged_since(cx: &Cx, last_modified: HttpDate) -> bool {
    let headers = headers(cx);
    !headers.contains_key(IF_NONE_MATCH)
        && headers
            .get(IF_MODIFIED_SINCE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<HttpDate>().ok())
            .is_some_and(|since| last_modified <= since)
}

/// Guesses a file's `Content-Type` from its extension, falling back to
/// `application/octet-stream`.
fn content_type(path: &FsPath) -> HeaderValue {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    // A guessed type is drawn from a fixed table of header-safe strings.
    HeaderValue::from_str(mime.as_ref()).expect("a guessed content type is a valid header value")
}

/// Streams the first `len` bytes of `file` as a response body, one chunk at a
/// time.
///
/// The body is cut at `len`, the length the response advertises, so a file
/// that grows while it is sent does not overrun its `Content-Length`.
fn stream(file: File, len: u64) -> Body {
    let chunks =
        ReaderStream::with_capacity(file.take(len), CHUNK_SIZE).map_ok(Frame::<Bytes>::data);
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

    /// Creates an empty directory for a test, wiping whatever an earlier run
    /// left behind.
    fn temp_dir(name: &str) -> FsPathBuf {
        let dir = env::temp_dir().join(format!("topcoat-router-directory-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Fills `dir` with a fixed set of files and a nested directory.
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

    /// Dispatches a request through the router and reads the full response.
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
    fn serves_from_the_root_prefix() {
        let dir = temp_dir("root");
        populate(&dir);
        let router = router("/{*file}", &dir);
        let (status, _, body) = get(&router, "/css/site.css");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "body{}");
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
        // Opening the pipe would block until a writer connects, so this only
        // completes if the route rejects it without opening it.
        let (status, _, _) = get(&router, "/public/pipe");
        assert_eq!(status, StatusCode::NOT_FOUND);
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
