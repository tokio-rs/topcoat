use std::{borrow::Cow, collections::HashMap, hash::Hash, marker::PhantomData, pin::Pin};

use http::Method;
use topcoat_core::runtime::{context::Cx, error::Result};

use crate::runtime::{Body, PathSegment, Response, Route};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ActionId(&'static str);

impl ActionId {
    pub const fn new(inner: &'static str) -> Self {
        Self(inner)
    }

    pub fn as_str(&self) -> &str {
        self.0
    }
}

pub type ActionHandlerFn =
    for<'cx> fn(
        cx: &'cx Cx,
        body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'cx>>;

#[derive(Debug, Clone)]
pub struct Action<A, R> {
    id: ActionId,
    handle: ActionHandlerFn,
    _phantom: PhantomData<fn(A) -> R>,
}

impl<A, R> Action<A, R> {
    pub const fn new(id: ActionId, handle: ActionHandlerFn) -> Self {
        Self {
            id,
            handle,
            _phantom: PhantomData,
        }
    }

    pub fn id(&self) -> ActionId {
        self.id
    }
}

#[derive(Debug, Clone)]
pub struct ErasedAction {
    id: ActionId,
    handle: ActionHandlerFn,
}

impl ErasedAction {
    pub fn id(&self) -> ActionId {
        self.id
    }

    pub async fn handle(&self, cx: &Cx, body: Body) -> Result<Response> {
        (self.handle)(cx, body).await
    }
}

impl<A, R> From<Action<A, R>> for ErasedAction {
    fn from(value: Action<A, R>) -> Self {
        Self {
            id: value.id,
            handle: value.handle,
        }
    }
}

impl From<ErasedAction> for Route {
    fn from(value: ErasedAction) -> Self {
        Self::new(
            Method::POST,
            Cow::Owned(
                [
                    PathSegment::Static("_topcoat"),
                    PathSegment::Static("actions"),
                    PathSegment::Static(value.id.0),
                ]
                .into_iter()
                .collect(),
            ),
            value.handle,
        )
    }
}
#[cfg(feature = "discover")]
inventory::collect!(ErasedAction);

#[derive(Clone, Default)]
pub struct Actions {
    actions: HashMap<ActionId, ErasedAction>,
}

impl Actions {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn register(&mut self, action: impl Into<ErasedAction>) {
        let action = action.into();
        self.actions.insert(action.id, action);
    }

    /// Returns `true` if no action has been registered.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

impl IntoIterator for Actions {
    type Item = ErasedAction;
    type IntoIter = std::collections::hash_map::IntoValues<ActionId, ErasedAction>;

    fn into_iter(self) -> Self::IntoIter {
        self.actions.into_values()
    }
}
