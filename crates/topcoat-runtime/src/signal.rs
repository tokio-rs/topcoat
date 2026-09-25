use std::{any::TypeId, collections::HashMap, panic::Location, sync::Arc};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
use topcoat_core::{
    context::{Cx, identity, try_request_context},
    identity::{Identity, SiteKey},
};
use topcoat_view::{HoistKey, hoist, hoist_once};

use crate::{Surrogate, Surrogated};

/// The identity of a signal, shared by the server and the browser runtime.
///
/// The context's identity and the `signal` call's location determine the id.
/// The same invocation therefore keeps its id across renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalId(u128);

impl SignalId {
    /// Derives the id at `location` below the context's checked identity.
    pub(crate) fn derive(identity: Identity, location: &Location<'_>) -> Self {
        Self(identity.child(SiteKey::from_location(location)).hash())
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
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = SignalId;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a hex signal id")
            }

            fn visit_str<E: serde::de::Error>(self, hex: &str) -> Result<Self::Value, E> {
                u128::from_str_radix(hex, 16)
                    .map(SignalId)
                    .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(hex), &self))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

/// Signal values sent by the client for a server render, keyed by signal id.
///
/// Register this in the request context to restore signals from client
/// state. [`signal`] uses the matching value if it fits the signal's type,
/// or computes an initial value otherwise. All restored values are untrusted.
#[derive(Debug, Default, Deserialize)]
#[serde(transparent)]
pub struct SignalValues(HashMap<SignalId, serde_json::Value>);

impl SignalValues {
    /// The value carried for `id`, if any.
    fn get(&self, id: SignalId) -> Option<&serde_json::Value> {
        self.0.get(&id)
    }
}

/// Reactive state shared by browser expressions.
///
/// Create a signal with [`signal`] during rendering. Runtime expressions
/// can read and write it. When its value changes, browser expressions
/// that read it run again. Cloning a signal is cheap and refers to the
/// same state.
///
/// On the server, [`get`](Self::get) and [`read`](Self::read) track the
/// signal as a dependency. A change re-renders the enclosing shard, or the
/// page if there is no shard. Use [`get_untracked`](Self::get_untracked) or
/// [`read_untracked`](Self::read_untracked) to read without this dependency.
/// **Validate values read on the server because the client can supply them.**
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
    /// A change re-renders the enclosing shard, or the page if there is no
    /// shard. Repeated reads create only one dependency.
    ///
    /// # Panics
    ///
    /// Panics outside an active rendering scope or runtime expression.
    #[must_use]
    #[track_caller]
    pub fn read(&self) -> &T {
        self.track();
        &self.value
    }

    /// Borrows the current value without tracking the signal.
    ///
    /// Outside runtime expressions, this creates no dependency and needs
    /// no rendering scope.
    #[must_use]
    pub fn read_untracked(&self) -> &T {
        crate::expr::mark_signal_read();
        &self.value
    }

    /// Hoists the marker that makes the enclosing body's content depend on
    /// this signal.
    #[track_caller]
    fn track(&self) {
        // Rust fallbacks inside expr! follow the client-reactive path too.
        if crate::expr::mark_signal_read() {
            return;
        }
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
    /// Uses the same tracking rules as [`read`](Self::read).
    ///
    /// # Panics
    ///
    /// Panics outside an active rendering scope or runtime expression.
    #[must_use]
    #[track_caller]
    pub fn get(&self) -> T {
        self.track();
        T::clone(&self.value)
    }

    /// Clones the current value without tracking the signal.
    ///
    /// Uses the same tracking rules as [`read_untracked`](Self::read_untracked).
    #[must_use]
    pub fn get_untracked(&self) -> T {
        T::clone(self.read_untracked())
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

/// A value that can be sent to the browser and restored from browser state.
///
/// Implemented automatically for runtime types whose surrogates support
/// serialization and deserialization.
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
/// The initial render calls `init` and sends its value to the browser.
/// Capture the signal in runtime expressions or pass it to components.
/// Clones refer to the same signal.
///
/// When a render resumes browser state, a matching value replaces `init`.
/// This preserves signal state across renders. **Validate values read on
/// the server** because the client can supply any value that fits the type.
///
/// ```rust
/// use topcoat::{Result, context::Cx, runtime::signal, view::*};
///
/// #[component]
/// async fn counter(cx: &Cx) -> Result<impl View> {
///     let count = signal(cx, || 0usize);
///
///     Ok(view! {
///         <button @click=$(|_e| count.increment())>"+1"</button>
///         <p>"Count: " $(count.get())</p>
///     })
/// }
/// ```
///
/// A signal's identity comes from `cx` and the location of this call.
/// Components receive their invocation's context automatically. Components
/// inside a template loop need `#[key(...)]` on that loop to distinguish
/// their signals.
///
/// For ordinary helpers called more than once, pass a context derived with
/// [`Cx::keyed`]. Separate `cx.keyed(())` calls distinguish source locations;
/// repeated calls at one location need distinct keys. Template loops do not
/// rebind context variables used in ordinary Rust expressions.
///
/// # Panics
///
/// Panics if `cx` belongs to a memoized call or carries an ambiguous identity,
/// or if no view is collecting signal declarations. A spawned task must
/// establish its own rendering scope before creating signals.
#[track_caller]
pub fn signal<T>(cx: &Cx, init: impl FnOnce() -> T) -> Signal<T>
where
    T: SignalValue,
{
    let id = SignalId::derive(identity(cx), Location::caller());
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

    use topcoat::view::{HoistView, ViewExt, internal::ThenView, view};
    use topcoat_core::context::with_identity;

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
        let cx = &with_identity(Cx::default(), Identity::ROOT.child(site));
        let view = HoistView::new(ThenView::new(async move {
            let signal = signal(cx, || 0.0_f64);
            Ok(view! { cx => <p>(signal.id().to_string())</p> })
        }));
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
        let cx = with_identity(
            Cx::default(),
            Identity::ROOT.ambiguous_child(SITE_A, "`card` at src/a.rs:1"),
        );
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
    fn context_keys_distinguish_repeated_helper_calls() {
        fn keyed_number(key: u32) -> SignalId {
            number_signal(&Cx::default().keyed(key)).0
        }

        let first = keyed_number(1);
        let second = keyed_number(2);
        assert_ne!(first, second);
        assert_eq!(keyed_number(2), second);
        assert_eq!(keyed_number(1), first);

        let cx = Cx::default();
        let first = number_signal(&cx.keyed(())).0;
        let second = number_signal(&cx.keyed(())).0;
        assert_ne!(first, second);
    }

    #[test]
    #[should_panic(expected = "identity cannot be read inside memoized functions")]
    fn memoized_functions_cannot_create_signals() {
        let cx = Cx::default();
        topcoat_core::context::memoize_cache(&cx)
            .memoize(&cx, (), (), |cx, ()| signal(&cx.keyed(()), || 0.0));
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
