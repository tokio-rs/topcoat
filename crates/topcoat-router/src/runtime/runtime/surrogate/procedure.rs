use std::pin::Pin;

use ref_cast::RefCast;
use serde::{Deserialize, Serialize, de};
use topcoat_core::runtime::{context::Cx, error::Result};
use topcoat_runtime::runtime::Surrogated;

use crate::runtime::{Body, ProcedureId, Response};

#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct Procedure<A, R>(crate::runtime::Procedure<A, R>);

impl<A, R> Procedure<A, R> {
    #[inline]
    pub(crate) const fn new(v: crate::runtime::Procedure<A, R>) -> Self {
        Self(v)
    }
}

impl<A, R> Procedure<A, R>
where
    A: Surrogated,
    R: Surrogated,
{
    pub async fn call(&self, _args: A::Surrogate) -> R::Surrogate {
        panic!("procedures cannot be executed on the server");
    }
}

topcoat_runtime::impl_surrogate!({A, R} crate::runtime::Procedure<A, R>, Procedure<A, R>);
topcoat_runtime::impl_surrogate_ref!({A, R} crate::runtime::Procedure<A, R>, Procedure<A, R>);
topcoat_runtime::impl_surrogate_mut!({A, R} crate::runtime::Procedure<A, R>, Procedure<A, R>);

impl<A, R> Serialize for Procedure<A, R> {
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

impl<'de, A, R> Deserialize<'de> for Procedure<A, R> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct TaggedProcedure {
            t: std::string::String,
            id: std::string::String,
        }

        let tagged = TaggedProcedure::deserialize(deserializer)?;
        if tagged.t != "Procedure" {
            return Err(de::Error::invalid_value(
                de::Unexpected::Str(&tagged.t),
                &"Procedure",
            ));
        }

        fn stub_handler<'cx>(
            _cx: &'cx Cx,
            _body: Body,
        ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'cx>> {
            Box::pin(async { panic!("deserialized procedures cannot be executed") })
        }

        let id: &'static str = Box::leak(tagged.id.into_boxed_str());
        Ok(Self::new(crate::runtime::Procedure::new(
            ProcedureId::new(id),
            stub_handler,
        )))
    }
}
