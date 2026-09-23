use http::HeaderMap;
use topcoat_core::context::Cx;
use topcoat_router::{
    Body, Layer, LayerFuture, Next, Path, RouterBuilder, response::response_headers,
};

use crate::{CookieJarCell, write_cookies};

/// A router layer that makes [`cookies`](crate::cookies) available to every
/// request and writes the pending cookie changes onto the response.
///
/// The changes are sent whether the handler returns a response or an error. A
/// cookie added before returning a redirect or an unauthorized error is set by
/// that response.
///
/// Usually installed with [`RouterBuilderCookieExt::cookies`].
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
    fn path(&self) -> Option<&Path> {
        Some(Path::new("/"))
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            let cx = cx.with(CookieJarCell::new());

            match next.run(&cx, body).await {
                Ok(mut response) => {
                    write_cookies(&cx, response.headers_mut());
                    Ok(response)
                }
                // The error's response does not exist yet, so the cookies
                // wait in the router's slot and land on it once it is built.
                Err(error) => {
                    let mut headers = HeaderMap::new();
                    write_cookies(&cx, &mut headers);
                    response_headers(&cx).extend(headers);
                    Err(error)
                }
            }
        })
    }
}

/// Extension trait that adds cookie support to a [`RouterBuilder`].
pub trait RouterBuilderCookieExt {
    /// Registers a [`CookieLayer`] at the root path.
    ///
    /// Call this after registering any other root layer that uses
    /// [`cookies`](crate::cookies), because the most recently registered root
    /// layer runs first.
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

    use http::{Method, Request, StatusCode, header};
    use topcoat_core::{context::Cx, error::Result};
    use topcoat_router::{
        Body, Methods, Path, Route, RouteFuture, RouteId, Router,
        error::{redirect, unauthorized},
        response::Response,
    };

    use crate::{Cookies, RouterBuilderCookieExt, cookies};

    /// What the route returns after adding its cookie.
    #[derive(Clone, Copy)]
    enum Outcome {
        Response,
        Redirect,
        Unauthorized,
    }

    /// Adds a cookie, then ends the request with the configured outcome.
    struct AddCookie(Outcome);

    impl Route for AddCookie {
        fn id(&self) -> RouteId {
            static ID: std::sync::LazyLock<RouteId> = std::sync::LazyLock::new(RouteId::new);
            *ID
        }

        fn methods(&self) -> Methods<'_> {
            Methods::Only(&[Method::GET])
        }

        fn path(&self) -> &Path {
            Path::new("/")
        }

        fn handle<'cx>(&'cx self, cx: &'cx Cx, _body: Body) -> RouteFuture<'cx> {
            Box::pin(async move {
                cookies(cx).add(("theme", "dark"));
                match self.0 {
                    Outcome::Response => Ok(Response::new(Body::empty())),
                    Outcome::Redirect => Err(redirect("/users").into()),
                    Outcome::Unauthorized => Err(unauthorized().into()),
                }
            })
        }
    }

    /// Sends a `GET /` through a router serving `route` behind the cookie
    /// layer and returns the status and `Set-Cookie` header of the response.
    async fn send(route: AddCookie) -> (StatusCode, Option<String>) {
        let router = Router::builder().route(route).cookies().build();
        let request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("request should build");

        let response = router.handle(request).await;

        let set_cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        (response.status(), set_cookie)
    }

    #[tokio::test]
    async fn layer_writes_pending_cookies() -> Result<()> {
        let (status, set_cookie) = send(AddCookie(Outcome::Response)).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(set_cookie.as_deref(), Some("theme=dark"));
        Ok(())
    }

    /// A redirect travels as an error, and its response is only built after
    /// the layer has run. The cookie has to be on that response all the same,
    /// or a handler cannot set a cookie and redirect in one step.
    #[tokio::test]
    async fn a_redirect_error_keeps_the_pending_cookies() -> Result<()> {
        let (status, set_cookie) = send(AddCookie(Outcome::Redirect)).await;

        assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(set_cookie.as_deref(), Some("theme=dark"));
        Ok(())
    }

    /// Any error response keeps the cookies, not only a redirect: a 401 that
    /// clears a stale session cookie is the case this guards.
    #[tokio::test]
    async fn an_error_response_keeps_the_pending_cookies() -> Result<()> {
        let (status, set_cookie) = send(AddCookie(Outcome::Unauthorized)).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(set_cookie.as_deref(), Some("theme=dark"));
        Ok(())
    }

    /// Hands an owned context handle back to the test, standing in for work
    /// that outlives the handler, such as a streaming body or a WebSocket task.
    /// The handler fails when `error` is set, so the test can check that the
    /// error path seals the jar too.
    struct Detach {
        handle: Arc<Mutex<Option<Cx>>>,
        error: bool,
    }

    impl Route for Detach {
        fn id(&self) -> RouteId {
            static ID: std::sync::LazyLock<RouteId> = std::sync::LazyLock::new(RouteId::new);
            *ID
        }

        fn methods(&self) -> Methods<'_> {
            Methods::Only(&[Method::GET])
        }

        fn path(&self) -> &Path {
            Path::new("/")
        }

        fn handle<'cx>(&'cx self, cx: &'cx Cx, _body: Body) -> RouteFuture<'cx> {
            Box::pin(async move {
                *self.handle.lock().expect("lock should not be poisoned") = Some(cx.clone());
                if self.error {
                    return Err(unauthorized().into());
                }
                Ok(Response::new(Body::empty()))
            })
        }
    }

    /// Runs a detaching route to completion and returns the handle it kept.
    async fn detach(error: bool) -> Cx {
        let handle = Arc::new(Mutex::new(None));
        let router = Router::builder()
            .route(Detach {
                handle: Arc::clone(&handle),
                error,
            })
            .cookies()
            .build();
        let request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("request should build");

        let _ = router.handle(request).await;
        let cx = handle.lock().expect("lock should not be poisoned").take();
        cx.expect("route should have stored a handle")
    }

    #[tokio::test]
    #[should_panic(expected = "cannot add a cookie after the response")]
    async fn writing_once_the_layer_is_done_panics() {
        // The layer has written its `Set-Cookie` headers by the time `handle`
        // returns, so this cookie could never reach the client.
        let cx = detach(false).await;
        cookies(&cx).add(("theme", "dark"));
    }

    #[tokio::test]
    #[should_panic(expected = "cannot add a cookie after the response")]
    async fn writing_once_the_layer_is_done_with_an_error_panics() {
        // The error path hands its cookies over just the same, so a write
        // after it would go nowhere as well.
        let cx = detach(true).await;
        cookies(&cx).add(("theme", "dark"));
    }
}
