use std::{hash::Hash, pin::Pin};

use serde::{Deserialize, Serialize};
use topcoat_core::{context::Cx, error::Result};
use topcoat_router::{
    Body, Method, Methods, Path, PathBuf, Route, RouteFuture, RouteId, RouterBuilder,
    response::Response,
};

use crate::{Surrogate, Surrogated};

const PROCEDURE_ROUTE_PREFIX: &str = "/_topcoat/procedures";

/// The identity of a procedure, stable across the server and the client
/// runtime.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProcedureId(&'static str);

impl ProcedureId {
    #[must_use]
    pub const fn new(inner: &'static str) -> Self {
        Self(inner)
    }

    #[must_use]
    fn as_str(&self) -> &str {
        self.0
    }
}

/// The future returned by [`Procedure::handle`]: a boxed, `Send` future
/// borrowing the procedure and its request context.
pub type ProcedureFuture<'cx> = Pin<Box<dyn Future<Output = Result<Response>> + Send + 'cx>>;

/// An async server function callable from the client runtime.
///
/// Registered into a [`RouterBuilder`] with
/// [`procedure`](RouterBuilderProcedureExt::procedure), which serves it as a
/// route dispatched by [`ProcedureId`].
pub trait Procedure: Send + Sync + 'static {
    /// The identity of this procedure.
    fn id(&self) -> ProcedureId;

    /// Handles a procedure call, deserializing its arguments from `body`.
    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> ProcedureFuture<'cx>;
}

impl<P: Procedure + ?Sized> Procedure for &'static P {
    fn id(&self) -> ProcedureId {
        (**self).id()
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> ProcedureFuture<'cx> {
        (**self).handle(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Procedure);

/// The argument and return types of a [`Procedure`], as seen by runtime
/// expressions calling it.
pub trait TypedProcedure: Procedure {
    /// The arguments, as a tuple in declaration order.
    type Args: Surrogated;

    /// The value a successful call resolves to.
    type Output: Surrogated;
}

/// A [`Route`] that handles calls to one server procedure.
pub struct ProcedureRoute {
    id: RouteId,
    path: PathBuf,
    procedure: Box<dyn Procedure>,
}

impl ProcedureRoute {
    /// Builds the route that serves `procedure`.
    pub fn new(procedure: impl Procedure) -> Self {
        Self {
            id: RouteId::new(),
            path: Path::new(&format!(
                "{PROCEDURE_ROUTE_PREFIX}/{}",
                procedure.id().as_str()
            ))
            .to_owned(),
            procedure: Box::new(procedure),
        }
    }
}

impl Route for ProcedureRoute {
    fn id(&self) -> RouteId {
        self.id
    }

    fn methods(&self) -> Methods<'_> {
        Methods::Only(&[Method::POST])
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        self.procedure.handle(cx, body)
    }
}

/// Registers server procedures on a [`RouterBuilder`].
pub trait RouterBuilderProcedureExt {
    /// Mounts a procedure route.
    #[must_use]
    fn procedure(self, procedure: impl Procedure) -> Self;

    /// Registers every procedure linked into the binary.
    #[cfg(feature = "discover")]
    #[must_use]
    fn discover_procedures(self) -> Self;
}

impl RouterBuilderProcedureExt for RouterBuilder {
    fn procedure(self, procedure: impl Procedure) -> Self {
        self.route(ProcedureRoute::new(procedure))
    }

    #[cfg(feature = "discover")]
    fn discover_procedures(mut self) -> Self {
        for &procedure in inventory::iter::<&'static dyn Procedure>() {
            self = self.procedure(procedure);
        }
        self
    }
}

/// The surrogate a [`Procedure`] value turns into inside a runtime
/// expression.
///
/// Serializes as the procedure's id, so the browser can call it back, and
/// exposes the typed [`call`](Self::call) that runtime expressions invoke.
pub struct ProcedureSurrogate<P>(P);

impl<P: TypedProcedure> ProcedureSurrogate<P> {
    #[must_use]
    pub const fn new(procedure: P) -> Self {
        Self(procedure)
    }

    /// Invokes the procedure from the client side.
    ///
    /// # Panics
    ///
    /// Always panics; procedures can only be invoked from the client runtime.
    #[allow(clippy::unused_async)]
    pub async fn call(
        &self,
        _args: <P::Args as Surrogated>::Surrogate,
    ) -> <P::Output as Surrogated>::Surrogate {
        panic!("procedures cannot be executed on the server");
    }
}

impl<P> Surrogate for ProcedureSurrogate<P>
where
    P: TypedProcedure + Surrogated<Surrogate = ProcedureSurrogate<P>>,
{
    type Real = P;

    fn into_real(self) -> Self::Real {
        self.0
    }
}

impl<P: TypedProcedure> Serialize for ProcedureSurrogate<P> {
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
