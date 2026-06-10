use std::pin::Pin;

use ref_cast::RefCast;
use serde::{Deserialize, Serialize, de};
use topcoat_core::runtime::{context::Cx, error::Result};
use topcoat_runtime::runtime::Surrogated;

use crate::runtime::{ActionId, Body, Response};

#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct Action<A, R>(crate::runtime::Action<A, R>);

impl<A, R> Action<A, R> {
    #[inline]
    pub(crate) const fn new(v: crate::runtime::Action<A, R>) -> Self {
        Self(v)
    }
}

impl<A, R> Action<A, R>
where
    A: Surrogated,
    R: Surrogated,
{
    pub async fn call(&self, _args: A::Surrogate) -> R::Surrogate {
        panic!("actions cannot be executed on the server");
    }
}

topcoat_runtime::impl_surrogate!({A, R} crate::runtime::Action<A, R>, Action<A, R>);
topcoat_runtime::impl_surrogate_ref!({A, R} crate::runtime::Action<A, R>, Action<A, R>);
topcoat_runtime::impl_surrogate_mut!({A, R} crate::runtime::Action<A, R>, Action<A, R>);

impl<A, R> Serialize for Action<A, R> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct TaggedAction<'a> {
            t: &'static str,
            id: &'a str,
        }

        TaggedAction {
            t: "Action",
            id: self.0.id().as_str(),
        }
        .serialize(serializer)
    }
}

impl<'de, A, R> Deserialize<'de> for Action<A, R> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct TaggedAction {
            t: std::string::String,
            id: std::string::String,
        }

        let tagged = TaggedAction::deserialize(deserializer)?;
        if tagged.t != "Action" {
            return Err(de::Error::invalid_value(
                de::Unexpected::Str(&tagged.t),
                &"Action",
            ));
        }

        fn stub_handler<'cx>(
            _cx: &'cx Cx,
            _body: Body,
        ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'cx>> {
            Box::pin(async { panic!("deserialized actions cannot be executed") })
        }

        let id: &'static str = Box::leak(tagged.id.into_boxed_str());
        Ok(Self::new(crate::runtime::Action::new(
            ActionId::new(id),
            stub_handler,
        )))
    }
}
