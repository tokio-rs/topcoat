#![doc = include_str!("../docs/serve_dir.md")]
#![cfg_attr(not(feature = "tower"), allow(rustdoc::broken_intra_doc_links))]

use std::{
    borrow::Cow,
    path::{Path as FsPath, PathBuf as FsPathBuf},
};

use http::{HeaderValue, Method, header::CONTENT_TYPE};
use topcoat_core::context::Cx;

use crate::{
    Body, IntoPath, Methods, Path, Route, RouteFuture, RouteId,
    error::not_found,
    request::{method, uri},
    response::Response,
    strip_prefix::{route_uri_prefix, strip_path_prefix},
};

/// GET and HEAD, the methods a directory of files can answer.
const METHODS: &[Method] = &[Method::GET, Method::HEAD];

/// A [`Route`] that serves files from a filesystem directory.
///
/// Mount it at a catch-all path with [`RouterBuilder::route`](crate::RouterBuilder::route).
/// The static prefix of that path is stripped from the request URI before the
/// remainder is joined onto the directory, so `ServeDir::new("/res/{*path}", "res")`
/// serves `res/hello.txt` at `/res/hello.txt`.
///
/// # Examples
///
/// ```rust,no_run
/// use topcoat::router::{Router, ServeDir};
///
/// let router = Router::builder()
///     .route(ServeDir::new("/res/{*path}", "res"))
///     .build();
/// ```
///
/// # Panics
///
/// [`new`](Self::new) panics if `path` is a string that is not a well-formed
/// route path.
#[derive(Debug, Clone)]
pub struct ServeDir {
    /// The identity of this route's handler.
    id: RouteId,
    /// The URL path this route handles.
    path: Cow<'static, Path>,
    /// Directory files are read from.
    root: FsPathBuf,
}

impl ServeDir {
    /// Serves the files in `root` at `path`.
    ///
    /// `path` is typically a catch-all (`/res/{*path}`). The segments before
    /// the first parameter are stripped from the request URI; the rest is
    /// joined onto `root`.
    #[must_use]
    #[track_caller]
    pub fn new(path: impl IntoPath, root: impl Into<FsPathBuf>) -> Self {
        Self {
            id: RouteId::new(),
            path: path.into_path(),
            root: root.into(),
        }
    }
}

impl Route for ServeDir {
    fn id(&self) -> RouteId {
        self.id
    }

    fn methods(&self) -> Methods<'_> {
        Methods::Only(METHODS)
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, _body: Body) -> RouteFuture<'cx> {
        let head = method(cx) == Method::HEAD;
        let relative = strip_path_prefix(uri(cx).path(), route_uri_prefix(&self.path))
            .unwrap_or_else(|| uri(cx).path().to_owned());
        Box::pin(async move {
            let Some(file) = resolve_file(&self.root, &relative).await else {
                return Err(not_found().into());
            };
            serve_file(&file, head).await
        })
    }
}

/// Resolves `url_path` under `root`, rejecting a path that would leave the
/// directory. A directory is mapped to `index.html` when that file exists.
async fn resolve_file(root: &FsPath, url_path: &str) -> Option<FsPathBuf> {
    let decoded = percent_encoding::percent_decode_str(url_path)
        .decode_utf8()
        .ok()?;
    let mut file = FsPathBuf::from(root);
    for component in decoded.split('/') {
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".." || component.contains('\0') {
            return None;
        }
        file.push(component);
    }
    let metadata = tokio::fs::metadata(&file).await.ok()?;
    if metadata.is_dir() {
        file.push("index.html");
        let metadata = tokio::fs::metadata(&file).await.ok()?;
        if metadata.is_file() { Some(file) } else { None }
    } else if metadata.is_file() {
        Some(file)
    } else {
        None
    }
}

async fn serve_file(file: &FsPath, head: bool) -> topcoat_core::error::Result<Response> {
    let content_type = mime_guess::from_path(file)
        .first_raw()
        .and_then(|mime| HeaderValue::from_str(mime).ok())
        .unwrap_or_else(|| HeaderValue::from_static("application/octet-stream"));

    let mut response = if head {
        Response::new(Body::empty())
    } else {
        let bytes = tokio::fs::read(file).await.map_err(|_| not_found())?;
        Response::new(Body::from(bytes))
    };
    response.headers_mut().insert(CONTENT_TYPE, content_type);
    Ok(response)
}

#[cfg(test)]
mod tests {
    use std::future::Future;

    use http::StatusCode;

    use super::*;
    use crate::{Router, request::Bytes, to_bytes};

    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    fn body_bytes(response: Response) -> Bytes {
        let (_, body) = response.into_parts();
        block_on(to_bytes(body, usize::MAX)).unwrap()
    }

    fn fixture_dir(name: &str) -> FsPathBuf {
        let dir =
            std::env::temp_dir().join(format!("topcoat-serve-dir-{name}-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("nested")).unwrap();
        std::fs::write(dir.join("hello.txt"), b"hello").unwrap();
        std::fs::write(dir.join("index.html"), b"index").unwrap();
        std::fs::write(dir.join("nested").join("file.txt"), b"nested").unwrap();
        dir
    }

    fn send(router: &Router, uri: &str) -> Response {
        let request = http::Request::builder()
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        block_on(router.handle(request))
    }

    #[test]
    fn serves_a_file_after_stripping_the_route_prefix() {
        let dir = fixture_dir("file");
        let router = Router::builder()
            .route(ServeDir::new("/res/{*path}", &dir))
            .build();

        let response = send(&router, "/res/hello.txt");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(&body_bytes(response)[..], b"hello");

        let response = send(&router, "/res/nested/file.txt");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(&body_bytes(response)[..], b"nested");
    }

    #[test]
    fn serves_index_html_for_a_directory() {
        let dir = fixture_dir("index");
        let router = Router::builder()
            .route(ServeDir::new("/res/{*path}", &dir))
            .build();

        let response = send(&router, "/res/nested");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        std::fs::write(dir.join("nested").join("index.html"), b"nested-index").unwrap();
        let response = send(&router, "/res/nested");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(&body_bytes(response)[..], b"nested-index");
    }

    #[test]
    fn rejects_a_path_that_leaves_the_directory() {
        let dir = fixture_dir("escape");
        std::fs::write(dir.parent().unwrap().join("secret.txt"), b"secret").ok();
        let router = Router::builder()
            .route(ServeDir::new("/res/{*path}", &dir))
            .build();

        let response = send(&router, "/res/%2e%2e/secret.txt");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
