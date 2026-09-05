use serde::{Deserialize, Serialize};
use topcoat_core::context::{Cx, request_arena};
use topcoat_view::hoist;
use uuid::Uuid;

use crate::Surrogated;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SignalId(Uuid);

impl SignalId {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SignalId {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SignalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A piece of state that lives in the browser.
///
/// A signal is created with [`signal`] during a server render and read or
/// written in runtime expressions, where it is reactive: an expression
/// re-runs in the browser whenever a signal it read changes. A signal is
/// handled by reference, which lives for the whole request, so any number
/// of runtime expressions can capture it and a component can take one as a
/// `&Signal<T>` prop.
#[derive(Debug)]
pub struct Signal<T> {
    id: SignalId,
    value: T,
}

impl<T> Signal<T> {
    #[inline]
    pub(crate) fn new(value: T) -> Self {
        Self {
            id: SignalId::new(),
            value,
        }
    }

    pub(crate) fn id(&self) -> SignalId {
        self.id
    }

    pub(crate) fn read(&self) -> &T {
        &self.value
    }

    /// Serializes the declaration the browser runtime creates the signal
    /// from: its id and initial value.
    fn declaration(&self) -> String
    where
        T: SignalValue,
    {
        #[derive(Serialize)]
        struct Declaration<'a, V>
        where
            V: ?Sized,
        {
            t: &'static str,
            id: std::string::String,
            v: &'a V,
        }

        let value = self.value.surrogate();
        let declaration = Declaration {
            t: "signal",
            id: self.id.to_string(),
            v: &value,
        };
        serde_json::to_string(&declaration).expect("failed to serialize signal declaration")
    }
}

impl<T> Signal<T>
where
    T: Clone,
{
    pub(crate) fn get(&self) -> T {
        self.value.clone()
    }
}

/// A value a signal can hold: one of the runtime's vocabulary types, which
/// can be serialized into the page for the browser to pick up.
///
/// Implemented for every type whose reference has a serializable surrogate;
/// there is nothing to implement by hand.
pub trait SignalValue {
    /// The serializable surrogate of a borrowed value.
    type Surrogate<'a>: Serialize
    where
        Self: 'a;

    /// Borrows the value as its surrogate.
    fn surrogate(&self) -> Self::Surrogate<'_>;
}

impl<T> SignalValue for T
where
    for<'a> &'a T: Surrogated,
    for<'a> <&'a T as Surrogated>::Surrogate: Serialize,
{
    type Surrogate<'a>
        = <&'a T as Surrogated>::Surrogate
    where
        Self: 'a;

    fn surrogate(&self) -> Self::Surrogate<'_> {
        self.into_surrogate()
    }
}

/// Creates a signal holding the value `init` returns.
///
/// The value is computed once, during the server render, and becomes the
/// signal's initial state in the browser. The signal lives on the request,
/// so the returned reference is valid for the rest of it: capture the signal
/// in as many runtime expressions as needed, or pass it on to components.
///
/// ```rust
/// use topcoat::{Result, context::Cx, runtime::signal, view::*};
///
/// #[component]
/// async fn counter(cx: &Cx) -> Result<impl View> {
///     let count = signal(cx, || 0.0);
///
///     Ok(view! {
///         <button @click=$(|_e| count.increment())>"+1"</button>
///         <p>"Count: " $(count.get())</p>
///     })
/// }
/// ```
///
/// A signal belongs to the page, layout, component, or shard body that
/// creates it, and is available to every runtime expression in that body's
/// view, including the components it renders.
///
/// # Panics
///
/// Panics when called outside a page, layout, component, or shard body,
/// including from work such a body spawns onto another task.
#[track_caller]
pub fn signal<T>(cx: &Cx, init: impl FnOnce() -> T) -> &Signal<T>
where
    T: SignalValue + Send + Sync + 'static,
{
    let signal = request_arena(cx).alloc(Signal::new(init()));
    let declaration = signal.declaration();
    hoist(move |parts| {
        parts.push_comment(|comment| {
            // The declaration carries untrusted application data, so it is
            // escaped like any other comment body rather than pushed raw.
            comment
                .push_promoted_str_unescaped(&"::topcoat::signal(")
                .push_string(declaration)
                .push_promoted_str_unescaped(&")");
        });
    });
    signal
}

#[cfg(test)]
mod tests {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        pin::pin,
        task::{Context, Poll, Waker},
    };

    use topcoat::view::{HoistView, ViewExt, internal::ThenView, view};

    use super::*;

    /// Drives a future that never yields to completion.
    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
                return output;
            }
        }
    }

    /// Renders a body creating one string signal, whose value is read back
    /// into the content.
    fn render_with_signal(value: &'static str) -> String {
        let cx = &Cx::default();
        let view = HoistView::new(ThenView::new(async move {
            let signal = signal(cx, || String::from(value));
            Ok(view! { cx => <p>(signal.read())</p> })
        }));
        block_on(view.single()).unwrap().render(cx)
    }

    #[test]
    fn creating_a_signal_outside_a_body_panics() {
        let cx = Cx::default();
        let panic = catch_unwind(AssertUnwindSafe(|| signal(&cx, || 0.0_f64))).unwrap_err();
        let message = panic.downcast::<&str>().expect("panics with a message");
        assert!(message.contains("no view is collecting hoisted parts"));
    }

    #[test]
    fn the_declaration_renders_ahead_of_the_content() {
        let html = render_with_signal("x");
        assert!(html.starts_with("<!--::topcoat::signal("), "{html}");
        assert!(html.ends_with("--><p>x</p>"), "{html}");
    }

    #[test]
    fn payload_cannot_terminate_the_comment() {
        // A value carrying `-->`, a quote, and an ampersand: the characters
        // that could break out of the comment or corrupt its JSON payload.
        let html = render_with_signal("a-->b\"c&d");

        // The comment context escaped `>`, so the only `-->` left is the
        // marker's own terminator; the payload cannot end the comment early.
        assert_eq!(html.matches("-->").count(), 1, "{html}");
        assert!(html.contains("--&gt;"), "{html}");
        // The JSON's own quotes round-trip as entities the client decodes.
        assert!(html.contains("&quot;"), "{html}");
    }
}
