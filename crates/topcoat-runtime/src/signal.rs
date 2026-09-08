use std::{any::TypeId, collections::HashMap, panic::Location, sync::Arc};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
use topcoat_core::context::{Cx, try_request_context};
use topcoat_view::{
    HoistKey, hoist, hoist_once,
    identity::{Identity, SiteKey},
};

use crate::{Surrogate, Surrogated};

/// The identity of a signal, shared by the server and the browser runtime.
///
/// An id is derived from the identity of the component body that created
/// the signal and the location of the `signal` call inside it, so the same
/// call reached through the same chain of invocations produces the same id
/// on every render. On the wire it is the hash as fixed-width hex, which
/// survives JSON where a 128 bit integer would not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalId(u128);

impl SignalId {
    /// Derives the id of the signal created at `location` inside the running
    /// component body.
    ///
    /// # Panics
    ///
    /// Panics if the running body's identity is ambiguous, meaning an
    /// invocation on the chain above it repeats without a `key` argument.
    #[track_caller]
    pub(crate) fn derive(location: &Location<'_>) -> Self {
        Self(
            Identity::current()
                .child(SiteKey::from_location(location))
                .hash(),
        )
    }
}

impl std::fmt::Display for SignalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

impl Serialize for SignalId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for SignalId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let hex = <&str>::deserialize(deserializer)?;
        u128::from_str_radix(hex, 16).map(Self).map_err(|_| {
            serde::de::Error::invalid_value(serde::de::Unexpected::Str(hex), &"a hex signal id")
        })
    }
}

/// The current values of signals a client sends along with a run, keyed by
/// signal id.
///
/// Registered on the request context of a run that resumes state, such as
/// a shard re-render: [`signal`] picks up the value stored under its id
/// instead of computing a fresh one. The values are chosen by the client,
/// so a value that does not fit the signal's type is ignored.
#[derive(Debug, Default, Deserialize)]
#[serde(transparent)]
pub struct SignalValues(HashMap<SignalId, serde_json::Value>);

impl SignalValues {
    /// The value carried for `id`, if any.
    fn get(&self, id: SignalId) -> Option<&serde_json::Value> {
        self.0.get(&id)
    }
}

/// A piece of state that lives in the browser.
///
/// A signal is created with [`signal`] during a server render and read or
/// written in runtime expressions, where it is reactive: an expression
/// re-runs in the browser whenever a signal it read changes. A signal is
/// cheap to clone, and every clone is the same signal: runtime expressions
/// clone the signals they capture, so any number of them can capture one,
/// and a component takes one as a `&Signal<T>` prop.
///
/// A signal can also be read on the server, outside any runtime
/// expression. [`get`](Self::get) and [`read`](Self::read) are tracked
/// reads: they make the body's content depend on the signal, so that a
/// change re-runs the page, or the innermost shard enclosing the read, with
/// the signal's current value. [`get_untracked`](Self::get_untracked) and
/// [`read_untracked`](Self::read_untracked) read the value without that
/// dependency. **Every value read on the server is untrusted user input**,
/// chosen by the client like a shard argument.
#[derive(Debug)]
pub struct Signal<T> {
    id: SignalId,
    /// Shared between clones, so capturing a signal never copies its value.
    value: Arc<T>,
}

impl<T> Signal<T> {
    #[inline]
    pub(crate) fn new(id: SignalId, value: T) -> Self {
        Self {
            id,
            value: Arc::new(value),
        }
    }

    pub(crate) fn id(&self) -> SignalId {
        self.id
    }

    /// Borrows the current value, tracking the signal as a dependency of
    /// the body reading it.
    ///
    /// The body's content is marked as depending on the signal, so the
    /// browser runtime re-runs the page, or the innermost shard enclosing
    /// the read, when the signal changes. Reading the same signal any number
    /// of times in one body marks its content once.
    ///
    /// # Panics
    ///
    /// Panics when called outside a page, layout, component, or shard body,
    /// like [`signal`].
    #[must_use]
    #[track_caller]
    pub fn read(&self) -> &T {
        self.track();
        &self.value
    }

    /// Borrows the current value without tracking the signal.
    ///
    /// A change to the signal does not re-run the body that read it this
    /// way. It can be called anywhere, not just inside a body.
    #[must_use]
    pub fn read_untracked(&self) -> &T {
        &self.value
    }

    /// Hoists the marker that makes the enclosing body's content depend on
    /// this signal.
    #[track_caller]
    fn track(&self) {
        let id = self.id;
        hoist_once(
            HoistKey::new((TypeId::of::<SignalId>(), id)),
            move |parts| {
                parts.push_comment(|comment| {
                    comment
                        .push_promoted_str_unescaped(&"::topcoat::dep(\"")
                        .push_string_unescaped(id.to_string())
                        .push_promoted_str_unescaped(&"\")");
                });
            },
        );
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
            id: SignalId,
            v: &'a V,
        }

        let value = self.value.surrogate();
        let declaration = Declaration {
            t: "signal",
            id: self.id,
            v: &value,
        };
        serde_json::to_string(&declaration).expect("failed to serialize signal declaration")
    }
}

impl<T> Signal<T>
where
    T: Clone,
{
    /// Clones the current value, tracking the signal as a dependency of
    /// the body reading it.
    ///
    /// This is [`read`](Self::read) for a value the body wants to own; the
    /// same tracking rules apply.
    ///
    /// # Panics
    ///
    /// Panics when called outside a page, layout, component, or shard body,
    /// like [`signal`].
    #[must_use]
    #[track_caller]
    pub fn get(&self) -> T {
        self.track();
        T::clone(&self.value)
    }

    /// Clones the current value without tracking the signal.
    ///
    /// This is [`read_untracked`](Self::read_untracked) for a value the
    /// body wants to own.
    #[must_use]
    pub fn get_untracked(&self) -> T {
        T::clone(&self.value)
    }
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            value: Arc::clone(&self.value),
        }
    }
}

/// A value a signal can hold: one of the runtime's vocabulary types, which
/// can be serialized into the page for the browser to pick up and read back
/// from what the browser sends.
///
/// Implemented for every type whose surrogate serializes and deserializes;
/// there is nothing to implement by hand.
pub trait SignalValue: Sized {
    /// The serializable surrogate of a borrowed value.
    type Surrogate<'a>: Serialize
    where
        Self: 'a;

    /// Borrows the value as its surrogate.
    fn surrogate(&self) -> Self::Surrogate<'_>;

    /// Reads a value back from the surrogate a client sent, or `None` if
    /// the surrogate does not fit this type.
    fn from_value(value: &serde_json::Value) -> Option<Self>;
}

impl<T> SignalValue for T
where
    T: Surrogated,
    T::Surrogate: DeserializeOwned,
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

    fn from_value(value: &serde_json::Value) -> Option<Self> {
        T::Surrogate::deserialize(value)
            .ok()
            .map(Surrogate::into_real)
    }
}

/// Creates a signal holding the value `init` returns.
///
/// The value is computed once, during the server render, and becomes the
/// signal's initial state in the browser. The returned signal is an
/// ordinary value the body keeps: capture it in as many runtime expressions
/// as needed, which clone it, or pass it on to components as `&Signal<T>`.
///
/// **A signal's value on the server is untrusted user input.** A run that
/// resumes state, such as a shard re-render or a page re-run, carries the
/// current values of the signals the client holds. When one of them is this
/// signal's, the signal starts from that value and `init` does not run, so
/// state created inside a shard survives its re-renders. The client chooses
/// those values and can send anything that fits the signal's type, so
/// validate a value read on the server before acting on it, like a shard
/// argument.
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
/// A signal's identity comes from the body that creates it and the location
/// of the call, so the same call reached the same way is the same signal on
/// every render. A body that renders repeatedly, such as a component
/// invoked in a `for` loop, needs a `key` argument on the invocation to tell
/// the repetitions' signals apart, and one body must not create two signals
/// from the same call site.
///
/// # Panics
///
/// Panics when called outside a page, layout, component, or shard body,
/// including from work such a body spawns onto another task, and when the
/// enclosing body's identity is ambiguous because an invocation above it
/// repeats without a `key`.
#[track_caller]
pub fn signal<T>(cx: &Cx, init: impl FnOnce() -> T) -> Signal<T>
where
    T: SignalValue,
{
    let id = SignalId::derive(Location::caller());
    let value = try_request_context::<SignalValues>(cx)
        .and_then(|values| values.get(id))
        .and_then(T::from_value)
        .unwrap_or_else(init);
    let signal = Signal::new(id, value);
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
        sync::{
            Arc, OnceLock,
            atomic::{AtomicBool, Ordering},
        },
        task::{Context, Poll, Waker},
    };

    use topcoat::view::{
        HoistView, ViewExt,
        identity::{IdentityGuard, IdentityView},
        internal::ThenView,
        view,
    };

    use super::*;

    const SITE_A: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);
    const SITE_B: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);

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
            Ok(view! { cx => <p>(signal.read_untracked())</p> })
        }));
        block_on(view.single()).unwrap().render(cx)
    }

    /// Renders a body creating one number signal and reading it with
    /// `read`, returning the content and the signal's dependency marker.
    fn render_reading(read: impl Fn(&Signal<String>) + Send + 'static) -> (String, String) {
        let cx = &Cx::default();
        let id = Arc::new(OnceLock::new());
        let out = Arc::clone(&id);
        let view = HoistView::new(ThenView::new(async move {
            let signal = signal(cx, || String::from("x"));
            read(&signal);
            out.set(signal.id()).unwrap();
            Ok(view! { cx => <p></p> })
        }));
        let html = block_on(view.single()).unwrap().render(cx);
        let marker = format!("<!--::topcoat::dep(\"{}\")-->", id.get().unwrap());
        (html, marker)
    }

    /// Renders a body creating one signal under the component identity at
    /// `site`, returning the signal's id.
    fn signal_id_at(site: SiteKey) -> SignalId {
        let cx = &Cx::default();
        let identity = IdentityGuard::enter(site).identity();
        let view = IdentityView::new(
            identity,
            HoistView::new(ThenView::new(async move {
                let signal = signal(cx, || 0.0_f64);
                Ok(view! { cx => <p>(signal.id().to_string())</p> })
            })),
        );
        let html = block_on(view.single()).unwrap().render(cx);
        let start = html.find("<p>").unwrap() + 3;
        let end = html.rfind("</p>").unwrap();
        SignalId(u128::from_str_radix(&html[start..end], 16).unwrap())
    }

    #[test]
    fn creating_a_signal_outside_a_body_panics() {
        let cx = Cx::default();
        let panic = catch_unwind(AssertUnwindSafe(|| signal(&cx, || 0.0_f64))).unwrap_err();
        let message = panic.downcast::<&str>().expect("panics with a message");
        assert!(message.contains("no view is collecting hoisted parts"));
    }

    #[test]
    fn the_same_call_site_renders_the_same_id_every_time() {
        assert_eq!(render_with_signal("x"), render_with_signal("x"));
        assert_eq!(signal_id_at(SITE_A), signal_id_at(SITE_A));
    }

    #[test]
    fn distinct_call_sites_render_distinct_ids() {
        let cx = &Cx::default();
        let view = HoistView::new(ThenView::new(async move {
            let first = signal(cx, || 0.0_f64);
            let second = signal(cx, || 0.0_f64);
            assert_ne!(first.id(), second.id());
            Ok(view! { cx => <p></p> })
        }));
        block_on(view.single()).unwrap();
    }

    #[test]
    fn distinct_identities_render_distinct_ids() {
        assert_ne!(signal_id_at(SITE_A), signal_id_at(SITE_B));
    }

    #[test]
    fn an_ambiguous_identity_panics() {
        let cx = Cx::default();
        let _guard = IdentityGuard::enter_ambiguous(SITE_A, "`card` at src/a.rs:1");
        let panic = catch_unwind(AssertUnwindSafe(|| signal(&cx, || 0.0_f64))).unwrap_err();
        let message = panic.downcast::<String>().expect("panics with a message");
        assert!(message.contains("`card` at src/a.rs:1"), "{message}");
    }

    /// Renders a body creating one number signal with an initial value of
    /// one, returning the signal's id and value and whether `init` ran.
    fn number_signal(cx: &Cx) -> (SignalId, f64, bool) {
        let seen = Arc::new(OnceLock::new());
        let out = Arc::clone(&seen);
        let view = HoistView::new(ThenView::new(async move {
            let init_ran = AtomicBool::new(false);
            let signal = signal(cx, || {
                init_ran.store(true, Ordering::Relaxed);
                1.0_f64
            });
            out.set((
                signal.id(),
                *signal.read_untracked(),
                init_ran.load(Ordering::Relaxed),
            ))
            .unwrap();
            Ok(view! { cx => <p></p> })
        }));
        block_on(view.single()).unwrap();
        *seen.get().unwrap()
    }

    /// A context carrying `value` for the signal `id`.
    fn cx_carrying(id: SignalId, value: serde_json::Value) -> Cx {
        Cx::default().with(SignalValues(HashMap::from([(id, value)])))
    }

    #[test]
    fn a_signal_resumes_from_the_value_the_request_carries() {
        let (id, value, init_ran) = number_signal(&Cx::default());
        assert_eq!((value, init_ran), (1.0, true));

        let cx = cx_carrying(id, serde_json::json!(5.0));
        assert_eq!(number_signal(&cx), (id, 5.0, false));
    }

    #[test]
    fn a_value_that_does_not_fit_the_signal_is_ignored() {
        let (id, ..) = number_signal(&Cx::default());

        let cx = cx_carrying(id, serde_json::json!("five"));
        assert_eq!(number_signal(&cx), (id, 1.0, true));
    }

    #[test]
    fn an_id_round_trips_through_json_as_hex() {
        let id = SignalId(0x1234_abcd);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{:032x}\"", 0x1234_abcd_u128));
        assert_eq!(serde_json::from_str::<SignalId>(&json).unwrap(), id);
        assert!(serde_json::from_str::<SignalId>("\"zz\"").is_err());
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

    #[test]
    fn a_tracked_read_renders_a_dependency_marker_after_the_declaration() {
        let (html, marker) = render_reading(|signal| {
            assert_eq!(signal.get(), "x");
        });
        assert!(html.starts_with("<!--::topcoat::signal("), "{html}");
        assert!(html.ends_with(&format!("{marker}<p></p>")), "{html}");
    }

    #[test]
    fn repeated_reads_render_one_dependency_marker() {
        let (html, marker) = render_reading(|signal| {
            let _ = signal.get();
            let _ = signal.read();
            let _ = signal.get();
        });
        assert_eq!(html.matches(&marker).count(), 1, "{html}");
    }

    #[test]
    fn untracked_reads_render_no_dependency_marker() {
        let (html, _) = render_reading(|signal| {
            assert_eq!(signal.get_untracked(), "x");
            assert_eq!(signal.read_untracked(), "x");
        });
        assert!(!html.contains("::topcoat::dep("), "{html}");
    }

    #[test]
    fn a_read_in_a_runtime_expression_renders_no_dependency_marker() {
        let (html, _) = render_reading(|signal| {
            let surrogate = crate::SignalSurrogate::new(signal.clone());
            let _ = surrogate.get();
            let _ = surrogate.read();
        });
        assert!(!html.contains("::topcoat::dep("), "{html}");
    }

    #[test]
    fn a_tracked_read_outside_a_body_panics() {
        let signal = Signal::new(SignalId(1), String::from("x"));
        let panic = catch_unwind(AssertUnwindSafe(|| signal.get())).unwrap_err();
        let message = panic.downcast::<&str>().expect("panics with a message");
        assert!(message.contains("no view is collecting hoisted parts"));
    }

    #[test]
    fn untracked_reads_work_outside_a_body() {
        let signal = Signal::new(SignalId(1), String::from("x"));
        assert_eq!(signal.get_untracked(), "x");
        assert_eq!(signal.read_untracked(), "x");
    }
}
