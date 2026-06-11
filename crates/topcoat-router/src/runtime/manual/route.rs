use std::pin::Pin;

use http::Method;
use topcoat_core::runtime::{context::Cx, error::Result};

use crate::runtime::{Body, Path, Response};

/// The future returned by [`Route::handle`].
pub type RouteHandlerFuture<'a> = Pin<Box<dyn Future<Output = Result<Response>> + Send + 'a>>;

/// An HTTP API handler bound to a method and path.
pub trait Route: Send + Sync + 'static {
    fn method(&self) -> Method;
    fn path(&self) -> &Path;
    fn handle<'a>(&'a self, cx: &'a Cx, body: Body) -> RouteHandlerFuture<'a>;
}

impl<R> Route for &'static R
where
    R: Route + ?Sized,
{
    #[inline]
    fn method(&self) -> Method {
        (*self).method()
    }

    #[inline]
    fn path(&self) -> &Path {
        (*self).path()
    }

    #[inline]
    fn handle<'a>(&'a self, cx: &'a Cx, body: Body) -> RouteHandlerFuture<'a> {
        (*self).handle(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Route);
