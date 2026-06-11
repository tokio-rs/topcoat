use std::{hash::Hash, pin::Pin};

use http::Method;
use topcoat_core::runtime::{context::Cx, error::Result};
use topcoat_runtime::runtime::Surrogated;

use crate::runtime::{Body, Path, PathBuf, PathSegment, Response, Route, RouteHandlerFuture};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ProcedureId(&'static str);

impl ProcedureId {
    #[inline]
    pub const fn new(inner: &'static str) -> Self {
        Self(inner)
    }

    #[inline]
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

pub type ProcedureHandlerFuture<'a> = Pin<Box<dyn Future<Output = Result<Response>> + Send + 'a>>;

/// An RPC handler reachable over `POST /_topcoat/procedures/{id}`.
///
/// Created by the `#[procedure]` macro, which generates a zero-sized type named
/// after the handler function and implements this trait for it.
pub trait Procedure: Send + Sync + 'static {
    fn id(&self) -> ProcedureId;
    fn handle<'a>(&'a self, cx: &'a Cx, body: Body) -> ProcedureHandlerFuture<'a>;
}

/// The typed view of a [`Procedure`], carrying its argument and return types
/// for the client-side `.call(..)`.
pub trait TypedProcedure: Procedure {
    type Args: Surrogated;
    type Return: Surrogated;
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Procedure);

/// Adapts a [`Procedure`] into a [`Route`] at `/_topcoat/procedures/{id}`.
#[derive(Clone)]
pub struct ProcedureRoute {
    procedure: &'static dyn Procedure,
    path: PathBuf,
}

impl ProcedureRoute {
    pub fn new(procedure: &'static dyn Procedure) -> Self {
        let path = [
            PathSegment::Static("_topcoat"),
            PathSegment::Static("procedures"),
            PathSegment::Static(procedure.id().as_str()),
        ]
        .into_iter()
        .collect();
        Self { procedure, path }
    }
}

impl Route for ProcedureRoute {
    fn method(&self) -> Method {
        Method::POST
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body) -> RouteHandlerFuture<'a> {
        self.procedure.handle(cx, body)
    }
}
