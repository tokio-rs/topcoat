use serde::Serialize;
use topcoat_router::Route;

use crate::{Surrogate, Surrogated};

/// A procedure's argument and return types for calls in runtime expressions.
///
/// Procedures implement [`Route`] to handle calls over HTTP. Register one
/// with [`RouterBuilder::route`](topcoat_router::RouterBuilder::route).
pub trait TypedProcedure: Route {
    /// The arguments, as a tuple in declaration order.
    type Args: Surrogated;

    /// The value a successful call resolves to.
    type Output: Surrogated;
}

/// A procedure captured by a runtime expression.
///
/// Serializes the endpoint URL for the browser and checks the argument and
/// return types of [`call`](Self::call).
pub struct ProcedureSurrogate<P> {
    procedure: P,
    /// The request URL, with route groups removed and `/` for the root.
    url: &'static str,
}

impl<P: TypedProcedure> ProcedureSurrogate<P> {
    /// Pairs a procedure with the URL that serves its calls.
    #[must_use]
    pub const fn new(procedure: P, url: &'static str) -> Self {
        Self { procedure, url }
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
        self.procedure
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
            path: &'static str,
        }

        TaggedProcedure {
            t: "Procedure",
            path: self.url,
        }
        .serialize(serializer)
    }
}
