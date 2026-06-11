use ref_cast::RefCast;
use serde::Serialize;
use topcoat_runtime::runtime::Surrogated;

use crate::runtime::ProcedureId;

#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct ProcedureSurrogate<A, R>(crate::runtime::Procedure<A, R>);

impl<A, R> ProcedureSurrogate<A, R> {
    #[inline]
    pub(crate) const fn new(v: crate::runtime::Procedure<A, R>) -> Self {
        Self(v)
    }
}

impl<A, R> ProcedureSurrogate<A, R>
where
    A: Surrogated,
    R: Surrogated,
{
    pub async fn call(&self, _args: A::Surrogate) -> R::Surrogate {
        panic!("procedures cannot be executed on the server");
    }
}

topcoat_runtime::impl_surrogate!({A, R} crate::runtime::Procedure<A, R>, ProcedureSurrogate<A, R>);
topcoat_runtime::impl_surrogate_ref!({A, R} crate::runtime::Procedure<A, R>, ProcedureSurrogate<A, R>);
topcoat_runtime::impl_surrogate_mut!({A, R} crate::runtime::Procedure<A, R>, ProcedureSurrogate<A, R>);

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
