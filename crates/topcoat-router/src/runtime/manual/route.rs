use std::{borrow::Cow, pin::Pin};

use http::Method;
use topcoat_core::runtime::{context::Cx, error::Result};

use crate::runtime::{Body, Path, Response};

pub trait Route: Send + Sync + 'static {
    fn method(&self) -> Method;
    fn path(&self) -> &Path;
    fn handle<'a>(
        &'a self,
        cx: &'a Cx,
        body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'a>>;
}

impl<R> Route for &R
where
    R: Route + ?Sized,
{
    #[inline]
    fn path(&self) -> &Path {
        (*self).path()
    }

    #[inline]
    fn method(&self) -> Method {
        (*self).method()
    }

    #[inline]
    fn handle<'a>(
        &'a self,
        cx: &'a Cx,
        body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'a>> {
        (*self).handle(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Route);
