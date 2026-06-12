use serde::{Deserialize, Serialize};
use topcoat_view::runtime::{NodeViewParts, Unescaped, ViewParts};
use uuid::Uuid;

use crate::runtime::Surrogated;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ClientHandleId(Uuid);

impl ClientHandleId {
    #[inline]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ClientHandleId {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ClientHandleId {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub struct ClientHandle<T> {
    id: ClientHandleId,
    value: T,
}

impl<T> ClientHandle<T> {
    #[inline]
    pub fn new(value: T) -> Self {
        Self {
            id: ClientHandleId::new(),
            value,
        }
    }

    #[inline]
    pub fn id(&self) -> ClientHandleId {
        self.id
    }

    #[inline]
    pub(crate) fn value(&self) -> &T {
        &self.value
    }
}

pub struct ClientHandleDeclaration<'a, T>(&'a ClientHandle<T>);

impl<'a, T> ClientHandleDeclaration<'a, T> {
    #[inline]
    pub fn new(handle: &'a ClientHandle<T>) -> Self {
        Self(handle)
    }
}

impl<T> NodeViewParts for ClientHandleDeclaration<'_, T>
where
    T: Surrogated,
    <T as Surrogated>::Surrogate: Serialize,
    for<'b> &'b T: Surrogated<Surrogate = &'b <T as Surrogated>::Surrogate>,
{
    fn into_view_parts(self, parts: &mut ViewParts) {
        parts.push(Unescaped::new_unchecked("<!-- ::topcoat::handle(\""));
        parts.push(Unescaped::new_unchecked(self.0.id().to_string()));
        parts.push(Unescaped::new_unchecked("\", "));
        parts.push(Unescaped::new_unchecked(
            serde_json::to_string((&self.0.value).into_surrogate()).unwrap(),
        ));
        parts.push(Unescaped::new_unchecked(") -->"));
    }
}
