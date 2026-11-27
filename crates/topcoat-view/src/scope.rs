use std::{
    cell::Cell,
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{Hoisted, PartsWriter, View, ViewBuffer, ViewFirst, ViewHandle, ViewSwap};

thread_local! {
    /// The state of the scope installed on the current thread, if any.
    static INSTALLED: Cell<Option<Box<ScopeState>>> = const { Cell::new(None) };
}

/// The state shared by the views of one build.
///
/// Whoever polls the outermost view owns the scope and keeps it between
/// polls. During a poll, views hand it to each other by mutable reference.
///
/// An async body cannot receive the scope as an argument, so the scope
/// polls the body itself with [`poll_first`](Self::poll_first) or
/// [`poll_swap`](Self::poll_swap). Inside the body, a [`DriveFuture`] hands
/// the scope on to the view it drives.
#[doc(hidden)]
#[derive(Debug)]
pub struct ViewScope {
    /// `None` only while the state is installed.
    state: Option<Box<ScopeState>>,
}

impl ViewScope {
    /// Creates the scope of a new build.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Some(Box::default()),
        }
    }

    /// Polls `body` for the first content of the view it drives and takes
    /// any content it yields.
    ///
    /// A yielded value accompanies `Poll::Pending`. With no yielded value,
    /// the body may still be waiting, or it may have completed or failed.
    pub(crate) fn poll_first<Fut: Future>(
        &mut self,
        body: Pin<&mut Fut>,
        cx: &mut Context<'_>,
    ) -> (Poll<Fut::Output>, Option<ViewFirst>) {
        let state = self.state();
        let outer_input = state.input.replace(DriveInput::First);
        let outer_yield = state.yield_first.take();

        let poll = {
            let _installed = self.install();
            body.poll(cx)
        };

        let state = self.state();
        state.input = outer_input;
        (poll, mem::replace(&mut state.yield_first, outer_yield))
    }

    /// Polls `body` for the next replacement of the view it drives and
    /// takes any replacement it yields.
    ///
    /// A yielded swap accompanies `Poll::Pending`. Completion of a driven
    /// view lets the body continue, so the body may yield another view's
    /// content or wait for other work before it finishes.
    pub(crate) fn poll_swap<Fut: Future>(
        &mut self,
        body: Pin<&mut Fut>,
        cx: &mut Context<'_>,
    ) -> (Poll<Fut::Output>, Option<ViewSwap>) {
        let state = self.state();
        let outer_input = state.input.replace(DriveInput::Swap);
        let outer_yield = state.yield_swap.take();

        let poll = {
            let _installed = self.install();
            body.poll(cx)
        };

        let state = self.state();
        state.input = outer_input;
        (poll, mem::replace(&mut state.yield_swap, outer_yield))
    }

    /// Appends one view's instruction block to the buffer of the build,
    /// filled by `f`, and returns the handle to the block.
    pub(crate) fn block(&mut self, f: impl FnOnce(&mut PartsWriter<'_>)) -> ViewHandle {
        self.buffer().block(f)
    }

    /// Returns the buffer the views of the build write into.
    pub(crate) fn buffer(&mut self) -> &mut ViewBuffer {
        &mut self.state().buffer
    }

    /// Makes `content` self-contained, so that it can be rendered and kept
    /// outside of the build.
    ///
    /// Content built in this scope takes the scope's buffer with it, and the
    /// scope continues with an empty one. Content that is self-contained
    /// already passes through.
    #[must_use]
    pub fn seal(&mut self, content: ViewHandle) -> ViewHandle {
        content.seal_from(&mut self.state().buffer)
    }

    /// Returns the collection of the view being polled, if one is
    /// collecting hoisted parts.
    pub(crate) fn hoisted(&mut self) -> Option<&mut Hoisted> {
        self.state().hoisted.as_mut()
    }

    /// Runs `f` and collects the parts hoisted meanwhile into `hoisted`.
    ///
    /// A collection that received the parts before receives them again
    /// afterwards.
    pub(crate) fn collecting<R>(
        &mut self,
        hoisted: &mut Hoisted,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let outer = self.state().hoisted.replace(mem::take(hoisted));
        let result = f(self);
        *hoisted = mem::replace(&mut self.state().hoisted, outer)
            .expect("the collection stays in its scope while it collects");
        result
    }

    /// Runs `f` with `inner`, the scope of a build nested in this one.
    ///
    /// Parts hoisted in the nested build land in the collection of this
    /// scope.
    pub(crate) fn nested<R>(&mut self, inner: &mut Self, f: impl FnOnce(&mut Self) -> R) -> R {
        inner.state().hoisted = self.state().hoisted.take();
        let result = f(inner);
        self.state().hoisted = inner.state().hoisted.take();
        result
    }

    /// Runs `f` with this scope installed on the current thread, as a build
    /// nested in the one whose scope was installed before.
    pub(crate) fn install_nested<R>(&mut self, f: impl FnOnce() -> R) -> R {
        match Self::reclaim() {
            Some(mut outer) => outer.nested(self, |inner| {
                let _installed = inner.install();
                f()
            }),
            None => {
                let _installed = self.install();
                f()
            }
        }
    }

    /// Installs this scope on the current thread until the returned guard
    /// drops.
    ///
    /// A scope installed before is restored at the same time.
    pub(crate) fn install(&mut self) -> Installed<'_> {
        self.state = INSTALLED.replace(self.state.take());
        Installed {
            scope: self,
            _not_send: PhantomData,
        }
    }

    /// Takes the scope installed on the current thread, or returns `None`
    /// if there is none.
    ///
    /// Dropping the returned scope installs it again.
    pub(crate) fn reclaim() -> Option<Reclaimed> {
        let state = INSTALLED.take()?;
        Some(Reclaimed {
            scope: Self { state: Some(state) },
            _not_send: PhantomData,
        })
    }

    /// Returns the state of this scope.
    fn state(&mut self) -> &mut ScopeState {
        self.state
            .as_mut()
            .expect("a scope is not used while it is installed")
    }
}

impl Default for ViewScope {
    fn default() -> Self {
        Self::new()
    }
}

/// What a [`ViewScope`] holds. It is boxed so that installing a scope moves
/// one pointer.
#[derive(Debug, Default)]
struct ScopeState {
    /// The buffer the views of the build write into.
    buffer: ViewBuffer,
    /// The parts hoisted into the view being polled, if one is collecting.
    hoisted: Option<Hoisted>,
    /// The view method the drives of the innermost polled body call.
    input: Option<DriveInput>,
    /// First content yielded to the innermost first-content poll.
    yield_first: Option<ViewFirst>,
    /// A replacement yielded to the innermost swap poll.
    yield_swap: Option<ViewSwap>,
}

/// The view method to call while driving the current body.
#[derive(Debug, Clone, Copy)]
enum DriveInput {
    First,
    Swap,
}

/// Keeps a [`ViewScope`] installed and returns its state to it on drop, also
/// when the code in between panics.
#[must_use = "the scope is uninstalled when the guard drops"]
pub(crate) struct Installed<'a> {
    scope: &'a mut ViewScope,
    /// The guard restores a thread local, so it must stay on the thread it
    /// was created on.
    _not_send: PhantomData<*const ()>,
}

impl Drop for Installed<'_> {
    fn drop(&mut self) {
        self.scope.state = INSTALLED.replace(self.scope.state.take());
    }
}

/// A [`ViewScope`] taken out of its installation, which is installed again
/// on drop.
pub(crate) struct Reclaimed {
    scope: ViewScope,
    /// The scope goes back into a thread local, so it must stay on the
    /// thread it was taken on.
    _not_send: PhantomData<*const ()>,
}

impl Deref for Reclaimed {
    type Target = ViewScope;

    fn deref(&self) -> &ViewScope {
        &self.scope
    }
}

impl DerefMut for Reclaimed {
    fn deref_mut(&mut self) -> &mut ViewScope {
        &mut self.scope
    }
}

impl Drop for Reclaimed {
    fn drop(&mut self) {
        INSTALLED.set(self.scope.state.take());
    }
}

pin_project! {
    /// A future that drives a view from inside an async body and yields the
    /// view's content to the scope polling that body.
    ///
    /// Await it only in a body polled through [`ViewScope::poll_first`] or
    /// [`ViewScope::poll_swap`]. Which of the two polls the body selects the
    /// view method directly; the drive does not track whether the view has
    /// emitted first content or reported itself as no longer live.
    ///
    /// Yielding content leaves the future pending. It resolves when the
    /// view reports no more swaps, or returns an error from either method.
    /// If another drive has filled the slot, this drive leaves its view
    /// unpolled and wakes the task so it can try again after the slot is read.
    pub(crate) struct DriveFuture<V> {
        #[pin]
        view: V,
    }
}

impl<V: View> DriveFuture<V> {
    /// Wraps a view to drive when the returned future is awaited.
    pub(crate) fn new(view: V) -> Self {
        Self { view }
    }
}

impl<V: View> Future for DriveFuture<V> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let Some(mut scope) = ViewScope::reclaim() else {
            panic!("polled DriveFuture without an installed ViewScope");
        };
        let Some(input) = scope.state().input else {
            panic!("polled DriveFuture without DriveInput");
        };

        match input {
            DriveInput::First => {
                if scope.state().yield_first.is_some() {
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                match this.view.poll_first(cx, &mut scope) {
                    Poll::Ready(Ok(first)) => scope.state().yield_first = Some(first),
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Pending => return Poll::Pending,
                }
            }
            DriveInput::Swap => {
                if scope.state().yield_swap.is_some() {
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                match this.view.poll_swap(cx, &mut scope) {
                    Poll::Ready(Ok(Some(swap))) => scope.state().yield_swap = Some(swap),
                    Poll::Ready(Ok(None)) => return Poll::Ready(Ok(())),
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Pending => return Poll::Pending,
                }
            }
        }

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use topcoat_core::context::Cx;

    use super::*;

    /// Creates a scope with a drive input set, to tell it apart from a new
    /// one.
    fn marked_scope() -> ViewScope {
        let mut scope = ViewScope::new();
        scope.state().input = Some(DriveInput::First);
        scope
    }

    /// Returns whether `scope` carries the mark of [`marked_scope`].
    fn is_marked(scope: &mut ViewScope) -> bool {
        scope.state().input.is_some()
    }

    #[test]
    fn nothing_can_be_reclaimed_without_an_installed_scope() {
        assert!(ViewScope::reclaim().is_none());
    }

    #[test]
    fn an_installed_scope_can_be_reclaimed_once_at_a_time() {
        let mut scope = ViewScope::new();
        let _installed = scope.install();

        let reclaimed = ViewScope::reclaim().expect("a scope is installed");
        assert!(ViewScope::reclaim().is_none());

        drop(reclaimed);
        assert!(ViewScope::reclaim().is_some());
    }

    #[test]
    fn changes_made_to_a_reclaimed_scope_reach_its_owner() {
        let mut scope = ViewScope::new();
        {
            let _installed = scope.install();
            let mut reclaimed = ViewScope::reclaim().expect("a scope is installed");
            reclaimed.state().input = Some(DriveInput::First);
        }

        assert!(ViewScope::reclaim().is_none());
        assert!(is_marked(&mut scope));
    }

    #[test]
    fn a_nested_install_restores_the_outer_scope() {
        let mut outer = marked_scope();
        let mut inner = ViewScope::new();
        let _outer = outer.install();

        {
            let _inner = inner.install();
            let mut reclaimed = ViewScope::reclaim().expect("a scope is installed");
            assert!(!is_marked(&mut reclaimed));
        }

        let mut reclaimed = ViewScope::reclaim().expect("a scope is installed");
        assert!(is_marked(&mut reclaimed));
    }

    #[test]
    fn a_scope_gets_its_state_back_when_the_installed_code_panics() {
        let mut scope = marked_scope();
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _installed = scope.install();
            panic!("boom");
        }));

        assert!(result.is_err());
        assert!(ViewScope::reclaim().is_none());
        assert!(is_marked(&mut scope));
    }

    #[test]
    fn sealed_content_renders_after_its_scope_is_gone() {
        let mut scope = ViewScope::new();
        let content = scope.block(|parts| {
            parts.push_str("a < b");
        });
        let content = scope.seal(content);
        drop(scope);

        assert_eq!(content.render(&Cx::new()), "a &lt; b");
    }

    #[test]
    fn a_scope_builds_again_after_its_content_was_sealed() {
        let mut scope = ViewScope::new();
        let first = scope.block(|parts| {
            parts.push_str("first");
        });
        let first = scope.seal(first);
        let second = scope.block(|parts| {
            parts.push_str("second");
        });
        let second = scope.seal(second);

        assert_eq!(first.render(&Cx::new()), "first");
        assert_eq!(second.render(&Cx::new()), "second");
    }
}
