use std::{borrow::Cow, hash::Hash, marker::PhantomData, pin::Pin};

use ref_cast::RefCast;
use serde::{Deserialize, Serialize};
use topcoat_core::runtime::{context::Cx, error::Result};
use topcoat_router::runtime::{
    Body, Method, Path, PathBuf, Response, Route, RouteFuture, RouterBuilder,
};

use crate::runtime::Surrogated;

const PROCEDURE_ROUTE_PREFIX: &str = "/_topcoat/procedures";

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProcedureId(&'static str);

impl ProcedureId {
    #[inline]
    #[must_use]
    pub const fn new(inner: &'static str) -> Self {
        Self(inner)
    }

    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0
    }
}

pub type ProcedureHandlerFn =
    for<'cx> fn(
        cx: &'cx Cx,
        body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'cx>>;

#[derive(Debug, Clone)]
pub struct Procedure<A, R> {
    id: ProcedureId,
    handle: ProcedureHandlerFn,
    _phantom: PhantomData<fn(A) -> R>,
}

impl<A, R> Procedure<A, R> {
    #[inline]
    pub const fn new(id: ProcedureId, handle: ProcedureHandlerFn) -> Self {
        Self {
            id,
            handle,
            _phantom: PhantomData,
        }
    }

    #[inline]
    #[must_use]
    pub fn id(&self) -> ProcedureId {
        self.id
    }
}

#[derive(Debug, Clone)]
pub struct ErasedProcedure {
    id: ProcedureId,
    handle: ProcedureHandlerFn,
}

impl ErasedProcedure {
    #[inline]
    #[must_use]
    pub const fn new<A, R>(procedure: &Procedure<A, R>) -> Self {
        Self {
            id: procedure.id,
            handle: procedure.handle,
        }
    }

    #[inline]
    #[must_use]
    pub fn id(&self) -> ProcedureId {
        self.id
    }

    /// Dispatches the procedure call, awaiting its handler future.
    ///
    /// # Errors
    ///
    /// Propagates any error returned by the underlying procedure handler.
    #[inline]
    pub async fn handle(&self, cx: &Cx, body: Body) -> Result<Response> {
        (self.handle)(cx, body).await
    }
}

impl<A, R> From<Procedure<A, R>> for ErasedProcedure {
    fn from(value: Procedure<A, R>) -> Self {
        Self {
            id: value.id,
            handle: value.handle,
        }
    }
}

impl<A, R> From<&Procedure<A, R>> for ErasedProcedure {
    fn from(value: &Procedure<A, R>) -> Self {
        Self::new(value)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(ErasedProcedure);

/// A [`Route`] that handles calls to one server procedure.
#[derive(Debug, Clone)]
pub struct ProcedureRoute {
    path: PathBuf,
    procedure: ErasedProcedure,
}

impl ProcedureRoute {
    /// Builds the route that serves `procedure`.
    pub fn new(procedure: impl Into<ErasedProcedure>) -> Self {
        let procedure = procedure.into();
        Self {
            path: Path::new(&format!(
                "{PROCEDURE_ROUTE_PREFIX}/{}",
                procedure.id().as_str()
            ))
            .to_owned(),
            procedure,
        }
    }
}

impl Route for ProcedureRoute {
    fn method(&self) -> Method {
        Method::POST
    }

    fn path(&self) -> Cow<'static, Path> {
        Cow::Owned(self.path.clone())
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        Box::pin(async move { self.procedure.handle(cx, body).await })
    }
}

/// Registers server procedures on a [`RouterBuilder`].
pub trait RouterBuilderProcedureExt {
    /// Mounts a procedure route at `/_topcoat/procedures/{id}`.
    #[must_use]
    fn procedure(self, procedure: impl Into<ErasedProcedure>) -> Self;

    /// Registers every procedure annotated with `#[procedure]` and collected at
    /// link time.
    #[cfg(feature = "discover")]
    #[must_use]
    fn discover_procedures(self) -> Self;
}

impl RouterBuilderProcedureExt for RouterBuilder {
    fn procedure(self, procedure: impl Into<ErasedProcedure>) -> Self {
        self.route(ProcedureRoute::new(procedure))
    }

    #[cfg(feature = "discover")]
    fn discover_procedures(mut self) -> Self {
        for procedure in inventory::iter::<ErasedProcedure>().cloned() {
            self = self.procedure(procedure);
        }
        self
    }
}

#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct ProcedureSurrogate<A, R>(Procedure<A, R>);

impl<A, R> ProcedureSurrogate<A, R> {
    #[inline]
    pub(crate) const fn new(v: Procedure<A, R>) -> Self {
        Self(v)
    }
}

impl<A, R> ProcedureSurrogate<A, R>
where
    A: Surrogated,
    R: Surrogated,
{
    /// Invokes the procedure from the client side.
    ///
    /// # Panics
    ///
    /// Always panics; procedures can only be invoked from the client runtime.
    #[allow(clippy::unused_async)]
    pub async fn call(&self, _args: A::Surrogate) -> R::Surrogate {
        panic!("procedures cannot be executed on the server");
    }
}

crate::impl_surrogate!({A, R} Procedure<A, R>, ProcedureSurrogate<A, R>);
crate::impl_surrogate_ref!({A, R} Procedure<A, R>, ProcedureSurrogate<A, R>);
crate::impl_surrogate_mut!({A, R} Procedure<A, R>, ProcedureSurrogate<A, R>);

impl<A, R> Serialize for ProcedureSurrogate<A, R> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct TaggedProcedure {
            t: &'static str,
            id: ProcedureId,
        }

        TaggedProcedure {
            t: "Procedure",
            id: self.0.id(),
        }
        .serialize(serializer)
    }
}
