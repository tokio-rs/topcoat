use std::borrow::Cow;

use topcoat_core::runtime::context::Cx;
use topcoat_router::runtime::{Body, Layer, LayerFuture, Next, Path, RouterBuilder};

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
    fn path(&self) -> Cow<'static, Path> {
        Cow::Borrowed(Path::new("/"))
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
    use std::borrow::Cow;

    use http::{Method, Request, header};
    use topcoat_core::runtime::{context::Cx, error::Result};
    use topcoat_router::runtime::{Body, Path, Response, Route, RouteFuture, Router};

    use crate::{Cookies, RouterBuilderCookieExt, cookies};

    struct AddCookie;

    impl Route for AddCookie {
        fn method(&self) -> Method {
            Method::GET
        }

        fn path(&self) -> Cow<'static, Path> {
            Cow::Borrowed(Path::new("/"))
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
}
