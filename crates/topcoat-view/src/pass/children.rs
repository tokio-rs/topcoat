use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::{Error, Result};

use crate::{
    identity::{Identity, IdentityGuard, IdentityKey, SiteKey},
    pass::{
        render::{Mount, RenderBuffer},
        scope::{ContentState, PassScope},
    },
};

/// The stored future of one live component.
pub type ComponentFuture<'f> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'f>>;

pin_project! {
    /// Installs a fixed identity around every poll of the wrapped future.
    pub(crate) struct InstallFuture<F> {
        #[pin]
        fut: F,
        identity: Identity,
    }
}

impl<F> InstallFuture<F> {
    pub(crate) fn new(identity: Identity, fut: F) -> Self {
        InstallFuture { fut, identity }
    }
}

impl<F: Future> Future for InstallFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<F::Output> {
        let this = self.project();
        let _guard = IdentityGuard::install(*this.identity);
        this.fut.poll(ctx)
    }
}

struct ChildEntry<'f> {
    identity: Identity,
    fut: Option<ComponentFuture<'f>>,
    catching: bool,
    content: bool,
    stashed: Option<Error>,
    advanced_pass: u64,
}

impl Drop for ChildEntry<'_> {
    fn drop(&mut self) {
        let hash = self.identity.hash();
        let stashed = self.stashed.is_some();
        let content = self.content;
        let _ = PassScope::try_with(|state| {
            state.births.remove(&hash);
            if stashed {
                state.stashed -= 1;
            }
            if content
                && let Some(content_state) = state.content.remove(&hash)
                && content_state.error.is_some()
            {
                state.stashed -= 1;
            }
        });
    }
}

/// Names a content component's output slot.
///
/// Returned by [`Children::content`], passed to a component as a prop, and
/// placed with [`RenderBuffer::place`]. The token is a plain name: the
/// content's state stays with its owner, so the token carries no borrow.
#[derive(Clone, Copy)]
pub struct ViewToken {
    pub(crate) identity: Identity,
}

/// A component's children, keyed by invocation identity.
///
/// A frame local of the generated render loop. Entries hold the child
/// futures across passes; dropping an entry is eviction, with the child's
/// cleanup running in ordinary drop order.
#[derive(Default)]
pub struct Children<'f> {
    map: HashMap<u128, ChildEntry<'f>>,
}

impl<'f> Children<'f> {
    #[must_use]
    pub fn new() -> Self {
        Children::default()
    }

    /// Advances the child invoked at `site` for the current pass, creating it
    /// on first reach. Emits the child's output marker into `out`.
    ///
    /// # Errors
    ///
    /// A child that failed is returned here, so the caller propagates it
    /// with `?` or catches it.
    ///
    /// # Panics
    ///
    /// Panics if the same identity is advanced twice in one pass, which means
    /// a repeated invocation is missing its `key`.
    pub fn advance<F>(
        &mut self,
        out: &mut RenderBuffer,
        site: SiteKey,
        mk: impl FnOnce() -> F,
    ) -> Result<()>
    where
        F: Future<Output = Result<()>> + Send + 'f,
    {
        self.advance_inner(out, Identity::current().child(site), false, mk)
    }

    /// Like [`Children::advance`], for an invocation that repeats: `key`
    /// tells the repetitions apart.
    ///
    /// # Errors
    ///
    /// A child that failed is returned here, so the caller propagates it
    /// with `?` or catches it.
    pub fn advance_keyed<F>(
        &mut self,
        out: &mut RenderBuffer,
        site: SiteKey,
        key: impl IdentityKey,
        mk: impl FnOnce() -> F,
    ) -> Result<()>
    where
        F: Future<Output = Result<()>> + Send + 'f,
    {
        self.advance_inner(out, Identity::current().keyed_child(site, key), false, mk)
    }

    /// Like [`Children::advance`], but marks the child as caught: an error
    /// that completes the child while this component is suspended is stashed
    /// and returned from the next advance instead of unwinding this
    /// component. This is the layout `slot` behavior.
    ///
    /// # Errors
    ///
    /// The child's failure, inline or stashed, is returned here for the
    /// caller to catch.
    pub fn advance_catching<F>(
        &mut self,
        out: &mut RenderBuffer,
        site: SiteKey,
        mk: impl FnOnce() -> F,
    ) -> Result<()>
    where
        F: Future<Output = Result<()>> + Send + 'f,
    {
        self.advance_inner(out, Identity::current().child(site), true, mk)
    }

    fn advance_inner<F>(
        &mut self,
        out: &mut RenderBuffer,
        identity: Identity,
        catching: bool,
        mk: impl FnOnce() -> F,
    ) -> Result<()>
    where
        F: Future<Output = Result<()>> + Send + 'f,
    {
        let pass = PassScope::with(|state| state.pass);
        let hash = identity.hash();
        let entry = self.map.entry(hash).or_insert_with(|| {
            PassScope::with(|state| state.births.insert(hash));
            ChildEntry {
                identity,
                fut: Some(Box::pin(InstallFuture::new(identity, mk()))),
                catching,
                content: false,
                stashed: None,
                advanced_pass: 0,
            }
        });
        assert!(
            entry.advanced_pass < pass,
            "a component was advanced twice in one pass: a repeated \
             invocation is missing its `key`",
        );
        entry.advanced_pass = pass;

        if let Some(error) = entry.stashed.take() {
            PassScope::with(|state| state.stashed -= 1);
            self.map.remove(&hash);
            return Err(error);
        }

        let waker = PassScope::with(|state| state.waker.clone())
            .expect("the driver sets the waker before every poll");
        let mut ctx = Context::from_waker(&waker);
        match poll_child(entry, &mut ctx) {
            Poll::Ready(Err(error)) => {
                self.map.remove(&hash);
                return Err(error);
            }
            Poll::Ready(Ok(())) | Poll::Pending => {}
        }

        out.child_marker(identity);
        Ok(())
    }

    /// Registers or advances the anonymous content component invoked at
    /// `site` and returns its placement token.
    ///
    /// Content is a child like any other: this component owns it, advances
    /// it every pass whether or not anyone places it, and sweeps it when its
    /// invocation disappears. Only its output travels differently, through
    /// the token a receiver places.
    ///
    /// # Errors
    ///
    /// A content error is delivered at placement. If nobody placed the token
    /// by the time the pass could seal, custody falls back here: the next
    /// advance returns the error, as if this component's own render failed.
    ///
    /// # Panics
    ///
    /// Panics if the same identity is advanced twice in one pass, which means
    /// a repeated invocation is missing its `key`.
    pub fn content<F>(&mut self, site: SiteKey, mk: impl FnOnce() -> F) -> Result<ViewToken>
    where
        F: Future<Output = Result<()>> + Send + 'f,
    {
        self.content_inner(Identity::current().child(site), mk)
    }

    /// Like [`Children::content`], for an invocation that repeats: `key`
    /// tells the repetitions apart.
    ///
    /// # Errors
    ///
    /// See [`Children::content`].
    pub fn content_keyed<F>(
        &mut self,
        site: SiteKey,
        key: impl IdentityKey,
        mk: impl FnOnce() -> F,
    ) -> Result<ViewToken>
    where
        F: Future<Output = Result<()>> + Send + 'f,
    {
        self.content_inner(Identity::current().keyed_child(site, key), mk)
    }

    fn content_inner<F>(&mut self, identity: Identity, mk: impl FnOnce() -> F) -> Result<ViewToken>
    where
        F: Future<Output = Result<()>> + Send + 'f,
    {
        let pass = PassScope::with(|state| state.pass);
        let hash = identity.hash();
        let entry = self.map.entry(hash).or_insert_with(|| {
            PassScope::with(|state| {
                state.births.insert(hash);
                state.content.insert(hash, ContentState::default());
            });
            ChildEntry {
                identity,
                fut: Some(Box::pin(InstallFuture::new(identity, mk()))),
                catching: false,
                content: true,
                stashed: None,
                advanced_pass: 0,
            }
        });
        assert!(
            entry.advanced_pass < pass,
            "a component was advanced twice in one pass: a repeated \
             invocation is missing its `key`",
        );
        entry.advanced_pass = pass;

        // Custody: an error no placement claimed on the pass it was recorded
        // is the owner's to handle.
        let custody = PassScope::with(|state| {
            let content_state = state.content.get_mut(&hash)?;
            if content_state.error_pass < pass {
                let error = content_state.error.take();
                if error.is_some() {
                    state.stashed -= 1;
                }
                error
            } else {
                None
            }
        });
        if let Some(error) = custody {
            self.map.remove(&hash);
            return Err(error);
        }

        let waker = PassScope::with(|state| state.waker.clone())
            .expect("the driver sets the waker before every poll");
        let mut ctx = Context::from_waker(&waker);
        if let Poll::Ready(Err(error)) = poll_child(entry, &mut ctx) {
            record_content_error(hash, error);
        }
        Ok(ViewToken { identity })
    }

    /// Evicts every child not advanced during the current pass. The generated
    /// render calls this after the view code ran, so a child orphaned by a
    /// branch switch is dropped, its bookkeeping cleaned by the entry's drop.
    pub fn sweep(&mut self) {
        let pass = PassScope::with(|state| state.pass);
        self.map.retain(|_, entry| entry.advanced_pass == pass);
    }
}

/// Records a content component's failure for a placement or the owner to
/// claim. Blocks the seal until claimed.
fn record_content_error(hash: u128, error: Error) {
    PassScope::with(|state| {
        let content_state = state
            .content
            .get_mut(&hash)
            .expect("a content component's state is registered at its birth");
        content_state.error = Some(error);
        content_state.error_pass = state.pass;
        state.stashed += 1;
    });
}

/// Polls one child once and updates the birth bookkeeping.
fn poll_child(entry: &mut ChildEntry<'_>, ctx: &mut Context<'_>) -> Poll<Result<()>> {
    let Some(fut) = entry.fut.as_mut() else {
        return Poll::Pending;
    };
    match fut.as_mut().poll(ctx) {
        Poll::Ready(result) => {
            entry.fut = None;
            PassScope::with(|state| state.births.remove(&entry.identity.hash()));
            match result {
                Err(error) => Poll::Ready(Err(error)),
                Ok(()) => panic!("a component future completed without suspending or failing"),
            }
        }
        Poll::Pending => Poll::Pending,
    }
}

/// Suspends a component until the next pass, driving its children while it
/// waits.
///
/// The boundary the generated render loop awaits between renders. While
/// suspended it keeps polling the children, so a birth parked on I/O at any
/// depth still makes progress. A child that completes with an error either
/// unwinds through this future's `Result` or is stashed if the child was
/// advanced catching.
pub fn pass_boundary<'a, 'f>(
    mount: &Mount,
    children: &'a mut Children<'f>,
) -> PassBoundary<'a, 'f> {
    PassBoundary {
        identity: mount.identity(),
        children,
    }
}

/// The future returned by [`pass_boundary`].
pub struct PassBoundary<'a, 'f> {
    identity: Identity,
    children: &'a mut Children<'f>,
}

impl Future for PassBoundary<'_, '_> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<()>> {
        let this = self.get_mut();
        let hash = this.identity.hash();
        let new_pass = PassScope::with(|state| {
            let rendered = state.slots.get(&hash).map_or(0, |slot| slot.rendered_pass);
            state.pass > rendered
        });
        if new_pass {
            return Poll::Ready(Ok(()));
        }
        for entry in this.children.map.values_mut() {
            if entry.stashed.is_some() {
                continue;
            }
            if let Poll::Ready(Err(error)) = poll_child(entry, ctx) {
                if entry.content {
                    record_content_error(entry.identity.hash(), error);
                } else if entry.catching {
                    entry.stashed = Some(error);
                    PassScope::with(|state| state.stashed += 1);
                } else {
                    return Poll::Ready(Err(error));
                }
            }
        }
        Poll::Pending
    }
}
