use std::{
    collections::VecDeque,
    fmt::{self, Write as _},
    pin::Pin,
    sync::{Mutex, MutexGuard},
    task::{Context, Poll},
};

use hashbrown::HashMap;
use serde::Serialize;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::{
    context::Cx,
    error::{Error, Result},
};

const RESERVED_KEY_PREFIX: &str = "@topcoat/";

#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientResourceKind {
    Stylesheet,
    Module,
}

#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientResource {
    pub key: String,
    pub kind: ClientResourceKind,
    pub url: String,
}

#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResponseEvent {
    Resource(ClientResource),
    Json { key: String, json: String },
}

#[doc(hidden)]
#[derive(Debug)]
pub struct ResponseEventReceiver {
    events: std::sync::Arc<ResponseEvents>,
    receiver: UnboundedReceiver<()>,
}

impl ResponseEventReceiver {
    #[must_use]
    pub fn try_next(&mut self) -> Option<ResponseEvent> {
        self.receiver.try_recv().ok()?;
        self.events.pop()
    }

    pub fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<ResponseEvent>> {
        match Pin::new(&mut self.receiver).poll_recv(cx) {
            Poll::Ready(Some(())) => Poll::Ready(self.events.pop()),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// The browser-side key for JSON sent through a request context.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct JsonKey(String);

impl JsonKey {
    /// Returns the key passed to `topcoat.json(key)` in browser code.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for JsonKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Default)]
struct State {
    events: VecDeque<ResponseEvent>,
    resources: HashMap<String, ClientResource>,
    json: HashMap<String, String>,
    response_id: Option<[u8; 16]>,
    next_json_key: u64,
}

#[derive(Debug)]
pub(crate) struct ResponseEvents {
    state: Mutex<State>,
    sender: UnboundedSender<()>,
    receiver: Mutex<Option<UnboundedReceiver<()>>>,
}

impl ResponseEvents {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = unbounded_channel();
        Self {
            state: Mutex::new(State::default()),
            sender,
            receiver: Mutex::new(Some(receiver)),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn push(&self, state: &mut State, event: ResponseEvent) {
        state.events.push_back(event);
        let _ = self.sender.send(());
    }

    pub(crate) fn has_events(&self) -> bool {
        !self.state().events.is_empty()
    }

    pub(crate) fn take_receiver(&self) -> UnboundedReceiver<()> {
        self.receiver
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
            .expect("response events were already consumed")
    }

    pub(crate) fn pop(&self) -> Option<ResponseEvent> {
        self.state().events.pop_front()
    }
}

impl Default for ResponseEvents {
    fn default() -> Self {
        Self::new()
    }
}

impl Cx {
    /// Sends JSON to browser code as soon as the response begins streaming.
    ///
    /// The returned key is unique within the response. Browser code can await
    /// the value with `topcoat.json(key)`.
    pub fn send_json<T>(&self, value: &T) -> Result<JsonKey>
    where
        T: Serialize + ?Sized,
    {
        let json = serde_json::to_string(value)?;
        let mut state = self.response_events.state();
        let response_id = if let Some(response_id) = state.response_id {
            response_id
        } else {
            let mut response_id = [0; 16];
            getrandom::fill(&mut response_id)
                .map_err(|error| Error::from(anyhow::anyhow!(error.to_string())))?;
            state.response_id = Some(response_id);
            response_id
        };
        let counter = state.next_json_key;
        state.next_json_key += 1;

        let mut key = String::with_capacity(43);
        key.push_str(RESERVED_KEY_PREFIX);
        for byte in response_id {
            write!(key, "{byte:02x}").expect("writing to a string cannot fail");
        }
        write!(key, "/{counter}").expect("writing to a string cannot fail");

        state.json.insert(key.clone(), json.clone());
        self.response_events.push(
            &mut state,
            ResponseEvent::Json {
                key: key.clone(),
                json,
            },
        );
        Ok(JsonKey(key))
    }

    /// Sends JSON under a stable key as soon as the response begins streaming.
    ///
    /// Reusing a key with the same value is deduplicated. Reusing it with a
    /// different value is an error. Keys beginning with `@topcoat/` are
    /// reserved for values created by [`send_json`](Self::send_json).
    pub fn send_json_named<T>(&self, key: impl Into<String>, value: &T) -> Result<JsonKey>
    where
        T: Serialize + ?Sized,
    {
        let key = key.into();
        if key.starts_with(RESERVED_KEY_PREFIX) {
            return Err(anyhow::anyhow!(
                "JSON keys beginning with {RESERVED_KEY_PREFIX:?} are reserved"
            )
            .into());
        }
        let json = serde_json::to_string(value)?;
        let mut state = self.response_events.state();
        if let Some(previous) = state.json.get(&key) {
            if previous == &json {
                return Ok(JsonKey(key));
            }
            return Err(
                anyhow::anyhow!("JSON key {key:?} was sent with two different values").into(),
            );
        }

        state.json.insert(key.clone(), json.clone());
        self.response_events.push(
            &mut state,
            ResponseEvent::Json {
                key: key.clone(),
                json,
            },
        );
        Ok(JsonKey(key))
    }

    #[doc(hidden)]
    pub fn require_client_resource(&self, resource: ClientResource) -> Result<()> {
        let mut state = self.response_events.state();
        if let Some(previous) = state.resources.get(&resource.key) {
            if previous == &resource {
                return Ok(());
            }
            return Err(anyhow::anyhow!(
                "client resource key {:?} describes two different resources",
                resource.key
            )
            .into());
        }

        state
            .resources
            .insert(resource.key.clone(), resource.clone());
        self.response_events
            .push(&mut state, ResponseEvent::Resource(resource));
        Ok(())
    }

    #[doc(hidden)]
    #[must_use]
    pub fn has_response_events(&self) -> bool {
        self.response_events.has_events()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn take_response_event_receiver(&self) -> ResponseEventReceiver {
        ResponseEventReceiver {
            events: std::sync::Arc::clone(&self.response_events),
            receiver: self.response_events.take_receiver(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::CxTestBuilder;

    #[test]
    fn generated_json_keys_are_unique_and_reserved() {
        let cx = CxTestBuilder::new().build();

        let first = cx.send_json(&1).unwrap();
        let second = cx.send_json(&2).unwrap();

        assert!(first.as_str().starts_with(RESERVED_KEY_PREFIX));
        assert!(second.as_str().starts_with(RESERVED_KEY_PREFIX));
        assert_ne!(first, second);
    }

    #[test]
    fn named_json_is_deduplicated_and_conflicts_are_rejected() {
        let cx = CxTestBuilder::new().build();

        cx.send_json_named("products", &[1, 2]).unwrap();
        cx.send_json_named("products", &[1, 2]).unwrap();
        let error = cx.send_json_named("products", &[3]).unwrap_err();

        assert!(error.to_string().contains("two different values"));
        assert!(matches!(
            cx.response_events.pop(),
            Some(ResponseEvent::Json { key, .. }) if key == "products"
        ));
        assert!(cx.response_events.pop().is_none());
    }

    #[test]
    fn named_json_cannot_use_generated_key_namespace() {
        let cx = CxTestBuilder::new().build();
        let error = cx.send_json_named("@topcoat/value", &1).unwrap_err();

        assert!(error.to_string().contains("reserved"));
    }
}
