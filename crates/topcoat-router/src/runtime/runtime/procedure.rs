use std::{hash::Hash, marker::PhantomData, pin::Pin};

use serde::{Deserialize, Serialize};
use topcoat_core::runtime::{context::Cx, error::Result};

use crate::runtime::{Body, Response};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProcedureId(&'static str);

impl ProcedureId {
    #[inline]
    pub const fn new(inner: &'static str) -> Self {
        Self(inner)
    }

    #[inline]
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
    pub const fn new<A, R>(procedure: &Procedure<A, R>) -> Self {
        Self {
            id: procedure.id,
            handle: procedure.handle,
        }
    }

    #[inline]
    pub fn id(&self) -> ProcedureId {
        self.id
    }

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

#[cfg(feature = "discover")]
inventory::collect!(ErasedProcedure);
