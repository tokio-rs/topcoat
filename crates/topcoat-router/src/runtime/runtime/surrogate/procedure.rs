use ref_cast::RefCast;
use serde::{Deserialize, Serialize, de};
use topcoat_runtime::runtime::{Surrogate, Surrogated};

use crate::runtime::TypedProcedure;

/// The client-side surrogate of a [`Procedure`](crate::runtime::Procedure).
///
/// Wraps the procedure's zero-sized type so its argument and return types are
/// available for `.call(..)`. On the server, calling panics; on the client it
/// performs the HTTP request.
#[derive(RefCast)]
#[repr(transparent)]
pub struct ProcedureSurrogate<P: TypedProcedure>(P);

impl<P: TypedProcedure> ProcedureSurrogate<P> {
    #[inline]
    pub const fn new(procedure: P) -> Self {
        Self(procedure)
    }

    /// Views a reference to the procedure as a reference to its surrogate.
    #[inline]
    pub fn from_ref(procedure: &P) -> &Self {
        Self::ref_cast(procedure)
    }

    pub async fn call(
        &self,
        _args: <P::Args as Surrogated>::Surrogate,
    ) -> <P::Return as Surrogated>::Surrogate {
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

impl<'a, P> Surrogate for &'a ProcedureSurrogate<P>
where
    P: TypedProcedure + Surrogated<Surrogate = ProcedureSurrogate<P>>,
    &'a P: Surrogated<Surrogate = &'a ProcedureSurrogate<P>>,
{
    type Real = &'a P;

    fn into_real(self) -> Self::Real {
        &self.0
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
            id: &'a str,
        }

        TaggedProcedure {
            t: "Procedure",
            id: self.0.id().as_str(),
        }
        .serialize(serializer)
    }
}

impl<'de, P> Deserialize<'de> for ProcedureSurrogate<P>
where
    P: TypedProcedure + Default,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct TaggedProcedure {
            t: std::string::String,
            #[allow(dead_code)]
            id: std::string::String,
        }

        let tagged = TaggedProcedure::deserialize(deserializer)?;
        if tagged.t != "Procedure" {
            return Err(de::Error::invalid_value(
                de::Unexpected::Str(&tagged.t),
                &"Procedure",
            ));
        }

        // The procedure's identity is its static type, so reconstruct directly.
        Ok(Self(P::default()))
    }
}
