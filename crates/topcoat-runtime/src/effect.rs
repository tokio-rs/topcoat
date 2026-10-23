use topcoat_core::context::Cx;
use topcoat_view::{
    Attribute, AttributeKey, AttributeValue, AttributeViewParts, HtmlContext, PartsWriter,
};

use crate::{BoolSurrogate, Event, EventTarget, Expr, Js, StrSurrogate};

/// Attaches a reactive browser effect to an element.
///
/// The callback runs after the inserted DOM has been hydrated and again when
/// a signal it reads changes. Its body never runs on the server. Attach one
/// effect per element using an attribute spread.
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     runtime::{EffectContext, effect, expr, signal},
///     view::{View, component, view},
/// };
///
/// #[component]
/// async fn panel(cx: &Cx) -> Result<impl View> {
///     let ready = signal(cx, || false);
///     let mounted = effect(expr!(|scope: EffectContext| {
///         ready.set(true);
///         scope.on_cleanup(|| ready.set(false));
///     }));
///     Ok(view! { <section (mounted)>"Panel"</section> })
/// }
/// ```
///
/// Cleanup runs before the next invocation and when the enclosing content is
/// replaced or disposed, including when replacement reuses the element.
/// Effects track synchronous reads only. They do not wait for pending server
/// requests, later live updates, layout, or paint.
///
/// Effect callbacks must be synchronous:
///
/// ```compile_fail
/// use topcoat::runtime::{EffectContext, effect, expr};
/// let hook = effect(expr!(async |_scope: EffectContext| {}));
/// ```
pub fn effect<F>(callback: Expr<F>) -> Effect
where
    F: FnOnce(EffectContext),
{
    Effect { js: callback.js }
}

/// An element attribute that installs a browser effect. See [`effect`].
pub struct Effect {
    js: Js,
}

impl AttributeViewParts for Effect {
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        Attribute::new("data-topcoat-effect", self.js).into_view_parts(cx, parts);
    }
}

impl IntoIterator for Effect {
    type Item = (AttributeKey, AttributeValue);
    type IntoIter = std::iter::Once<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once((
            AttributeKey::StaticStr {
                value: "data-topcoat-effect",
                context: HtmlContext::AttributeKey,
            },
            AttributeValue::String {
                value: self.js.to_source(),
                context: HtmlContext::AttributeValue,
            },
        ))
    }
}

/// Owns resources for one invocation of an [`effect`].
///
/// This browser-only facade is passed to compiled Rust callbacks. Raw
/// JavaScript can also use its `element` and `abortSignal` properties. Pass
/// `abortSignal` to listeners or requests and register other resources with
/// `on_cleanup`. Each invocation receives a fresh context.
#[derive(Clone)]
pub struct EffectContext {
    _private: (),
}

impl EffectContext {
    /// The element this effect is attached to, as a browser event target.
    #[must_use]
    pub fn target(&self) -> EventTarget {
        unreachable!("effect contexts are only available in the browser");
    }

    /// Attaches a synchronous listener owned by this invocation.
    ///
    /// The target may be outside the effect's DOM subtree. The listener is
    /// removed before rerun or disposal. Reads in the callback do not become
    /// effect dependencies. Listening after disposal has no effect.
    ///
    /// ```compile_fail
    /// use topcoat::runtime::{EffectContext, Event, effect, expr};
    /// let hook = effect(expr!(|scope: EffectContext| {
    ///     scope.listen(scope.target(), "reset", async |_event: Event| {});
    /// }));
    /// ```
    pub fn listen(
        &self,
        _target: EventTarget,
        _event: &StrSurrogate,
        _callback: impl FnOnce(Event) + 'static,
    ) {
        unreachable!("effect contexts are only available in the browser");
    }

    /// Registers a synchronous callback to run once when this invocation ends.
    ///
    /// Callbacks run in reverse registration order, without tracking signal
    /// reads. A failing callback is reported and remaining resources are still
    /// released. Registering after disposal runs the callback immediately.
    ///
    /// ```compile_fail
    /// use topcoat::runtime::{EffectContext, effect, expr};
    /// let hook = effect(expr!(|scope: EffectContext| {
    ///     scope.on_cleanup(async || {});
    /// }));
    /// ```
    pub fn on_cleanup(&self, _cleanup: impl FnOnce() + 'static) {
        unreachable!("effect contexts are only available in the browser");
    }

    /// Whether this invocation still owns its resources.
    ///
    /// Check this in deferred work before changing signals or the DOM. It
    /// becomes false before cleanup starts, on rerun or scope disposal.
    #[must_use]
    pub fn is_active(&self) -> BoolSurrogate {
        unreachable!("effect contexts are only available in the browser");
    }
}
