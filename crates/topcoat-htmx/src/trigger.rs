use http::HeaderValue;
use http::header::HeaderName;
use http::response::Parts;
use serde::Serialize;
use serde_json::{Map, Value};
use topcoat_core::{context::Cx, error::Result};
use topcoat_router::IntoResponseParts;

use crate::header;

/// A client-side event for htmx to trigger, optionally carrying a JSON detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HxEvent {
    /// The event name.
    pub name: String,
    /// An optional detail payload delivered to the event listener.
    pub data: Option<Value>,
}

impl HxEvent {
    /// Creates an event that triggers with no detail.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: None,
        }
    }

    /// Creates an event carrying `data` as its detail.
    ///
    /// # Errors
    ///
    /// Returns an error if `data` cannot be serialized to JSON.
    pub fn with_data(name: impl Into<String>, data: impl Serialize) -> Result<Self> {
        Ok(Self {
            name: name.into(),
            data: Some(serde_json::to_value(data)?),
        })
    }
}

impl From<&str> for HxEvent {
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl From<String> for HxEvent {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}

/// When htmx triggers the events of an [`HxResponseTrigger`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerTiming {
    /// Trigger as soon as the response is received (`HX-Trigger`).
    Receive,
    /// Trigger after the settle step (`HX-Trigger-After-Settle`).
    AfterSettle,
    /// Trigger after the swap step (`HX-Trigger-After-Swap`).
    AfterSwap,
}

impl TriggerTiming {
    fn header(self) -> HeaderName {
        match self {
            Self::Receive => header::HX_TRIGGER,
            Self::AfterSettle => header::HX_TRIGGER_AFTER_SETTLE,
            Self::AfterSwap => header::HX_TRIGGER_AFTER_SWAP,
        }
    }
}

/// Triggers client-side events via one of the `HX-Trigger` response headers.
///
/// When every event has no detail, the header is a comma-separated list of
/// names. As soon as one event carries data, the whole header is serialized as
/// a JSON object mapping each event name to its detail (or `null`).
///
/// # Examples
///
/// ```rust
/// use topcoat::htmx::{HxEvent, HxResponseTrigger};
///
/// // `HX-Trigger: refresh, close-modal`
/// let simple = HxResponseTrigger::receive(["refresh", "close-modal"]);
///
/// // JSON form, fired after the swap step.
/// let detailed =
///     HxResponseTrigger::after_swap([HxEvent::with_data("show-toast", "Saved!").unwrap()]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HxResponseTrigger {
    /// When the events fire.
    pub timing: TriggerTiming,
    /// The events to trigger.
    pub events: Vec<HxEvent>,
}

impl HxResponseTrigger {
    /// Triggers `events` with the given `timing`.
    pub fn new(
        timing: TriggerTiming,
        events: impl IntoIterator<Item = impl Into<HxEvent>>,
    ) -> Self {
        Self {
            timing,
            events: events.into_iter().map(Into::into).collect(),
        }
    }

    /// Triggers `events` as soon as the response is received (`HX-Trigger`).
    pub fn receive(events: impl IntoIterator<Item = impl Into<HxEvent>>) -> Self {
        Self::new(TriggerTiming::Receive, events)
    }

    /// Triggers `events` after the settle step (`HX-Trigger-After-Settle`).
    pub fn after_settle(events: impl IntoIterator<Item = impl Into<HxEvent>>) -> Self {
        Self::new(TriggerTiming::AfterSettle, events)
    }

    /// Triggers `events` after the swap step (`HX-Trigger-After-Swap`).
    pub fn after_swap(events: impl IntoIterator<Item = impl Into<HxEvent>>) -> Self {
        Self::new(TriggerTiming::AfterSwap, events)
    }

    /// Renders the header value, using the bare name list when no event carries
    /// a detail and the JSON object form otherwise.
    fn header_value(self) -> Result<HeaderValue> {
        if self.events.iter().all(|event| event.data.is_none()) {
            let names = self
                .events
                .iter()
                .map(|event| event.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Ok(HeaderValue::from_str(&names)?);
        }

        let object = self
            .events
            .into_iter()
            .map(|event| (event.name, event.data.unwrap_or(Value::Null)))
            .collect::<Map<_, _>>();
        Ok(HeaderValue::from_str(&serde_json::to_string(&object)?)?)
    }
}

impl IntoResponseParts for HxResponseTrigger {
    fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        let name = self.timing.header();
        parts.headers.insert(name, self.header_value()?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header_value(trigger: HxResponseTrigger) -> (HeaderName, String) {
        let name = trigger.timing.header();
        let mut parts = http::Response::new(()).into_parts().0;
        trigger
            .into_response_parts(&Cx::default(), &mut parts)
            .unwrap();
        let value = parts
            .headers
            .get(&name)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        (name, value)
    }

    #[test]
    fn name_only_events_join_with_commas() {
        let (name, value) = header_value(HxResponseTrigger::receive(["refresh", "close"]));
        assert_eq!(name, header::HX_TRIGGER);
        assert_eq!(value, "refresh, close");
    }

    #[test]
    fn events_with_data_serialize_as_json() {
        let trigger = HxResponseTrigger::after_swap([
            HxEvent::with_data("show-toast", "Saved!").unwrap(),
            HxEvent::new("refresh"),
        ]);
        let (name, value) = header_value(trigger);
        assert_eq!(name, header::HX_TRIGGER_AFTER_SWAP);
        let json: Value = serde_json::from_str(&value).unwrap();
        assert_eq!(json["show-toast"], "Saved!");
        assert_eq!(json["refresh"], Value::Null);
    }

    #[test]
    fn timing_selects_header() {
        let (name, _) = header_value(HxResponseTrigger::after_settle(["x"]));
        assert_eq!(name, header::HX_TRIGGER_AFTER_SETTLE);
    }
}
