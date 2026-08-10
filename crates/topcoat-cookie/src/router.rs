use topcoat_core::context::Cx;
use topcoat_router::{Body, Layer, LayerFuture, Next, Path, RouterBuilder};

use crate::{CookieJarCell, write_cookies};

/// A router layer that makes cookies available for the current request and
/// writes pending cookie changes onto the response.
#[derive(Debug, Clone, Copy, Default)]
pub struct CookieLayer;

impl CookieLayer {
    /// Creates a cookie layer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Layer for CookieLayer {
    fn path(&self) -> &Path {
        Path::new("/")
    }

    fn handle<'a>(&'a self, cx: &'a mut Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            cx.insert(CookieJarCell::new());

            let mut response = next.run(cx, body).await?;
            write_cookies(cx, response.headers_mut());
            Ok(response)
        })
    }
}

/// Installs cookie support on a [`RouterBuilder`].
///
/// Register it after other same-path layers that should be able to call
/// [`cookies`](crate::cookies), because the most recently registered root
/// layer runs first.
pub trait RouterBuilderCookieExt {
    /// Registers the root cookie layer.
    ///
    /// The layer stores the request's cookie jar in request context, parses the
    /// incoming `Cookie` headers on first access, and appends pending changes as
    /// `Set-Cookie` headers before the response is sent.
    #[must_use]
    fn cookies(self) -> Self;
}

impl RouterBuilderCookieExt for RouterBuilder {
    fn cookies(self) -> Self {
        self.layer(CookieLayer::new())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use http::{Method, Request, header};
    use topcoat_core::{context::Cx, error::Result};
    use topcoat_router::{Body, Methods, Path, Route, RouteFuture, Router, response::Response};

    use crate::{Cookies, RouterBuilderCookieExt, cookies};

    struct AddCookie;

    impl Route for AddCookie {
        fn methods(&self) -> Methods<'_> {
            Methods::Only(&[Method::GET])
        }

        fn path(&self) -> &Path {
            Path::new("/")
        }

        fn handle<'cx>(&'cx self, cx: &'cx Cx, _body: Body) -> RouteFuture<'cx> {
            Box::pin(async move {
                cookies(cx).add(("theme", "dark"));
                Ok(Response::new(Body::empty()))
            })
        }
    }

    #[tokio::test]
    async fn layer_writes_pending_cookies() -> Result<()> {
        let router = Router::builder().route(AddCookie).cookies().build();
        let request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("request should build");

        let response = router.handle(request).await;

        assert_eq!(
            response
                .headers()
                .get(header::SET_COOKIE)
                .and_then(|value| value.to_str().ok()),
            Some("theme=dark")
        );
        Ok(())
    }

    /// Hands an owned context handle back to the test, standing in for work
    /// that outlives the handler, such as a streaming body or a WebSocket task.
    struct Detach(Arc<Mutex<Option<Cx>>>);

    impl Route for Detach {
        fn methods(&self) -> Methods<'_> {
            Methods::Only(&[Method::GET])
        }

        fn path(&self) -> &Path {
            Path::new("/")
        }

        fn handle<'cx>(&'cx self, cx: &'cx Cx, _body: Body) -> RouteFuture<'cx> {
            Box::pin(async move {
                *self.0.lock().expect("lock should not be poisoned") = Some(cx.detach());
                Ok(Response::new(Body::empty()))
            })
        }
    }

    #[tokio::test]
    #[should_panic(expected = "cannot add a cookie after the response")]
    async fn writing_once_the_layer_is_done_panics() {
        let handle = Arc::new(Mutex::new(None));
        let router = Router::builder()
            .route(Detach(Arc::clone(&handle)))
            .cookies()
            .build();
        let request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("request should build");

        // The layer has written its `Set-Cookie` headers by the time `handle`
        // returns, so this cookie could never reach the client.
        let _ = router.handle(request).await;
        let cx = handle.lock().expect("lock should not be poisoned").take();
        cookies(cx.as_ref().expect("route should have detached")).add(("theme", "dark"));
    }
}
