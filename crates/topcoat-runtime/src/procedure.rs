use serde::Serialize;
use topcoat_router::Route;

use crate::{Surrogate, Surrogated};

/// The argument and return types of a procedure, as seen by runtime
/// expressions calling it.
///
/// A procedure is a [`Route`] serving calls at its path. Register it with
/// [`RouterBuilder::route`](topcoat_router::RouterBuilder::route) to expose
/// its HTTP endpoint.
pub trait TypedProcedure: Route {
    /// The arguments, as a tuple in declaration order.
    type Args: Surrogated;

    /// The value a successful call resolves to.
    type Output: Surrogated;
}

/// The surrogate a procedure value turns into inside a runtime expression.
///
/// Serializes as the path of the procedure's endpoint and exposes a typed
/// [`call`](Self::call) for browser expressions. A static reference lets
/// closures capture it without borrowing a local variable.
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

impl<P> Surrogate for &'static ProcedureSurrogate<P>
where
    P: TypedProcedure + Copy + Surrogated<Surrogate = Self>,
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
        struct TaggedProcedure<'a> {
            t: &'static str,
            path: &'a str,
        }

        TaggedProcedure {
            t: "Procedure",
            path: self.0.path().as_str(),
        }
        .serialize(serializer)
    }
}
