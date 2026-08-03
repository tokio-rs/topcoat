use core::{future::Future, pin::Pin};
use std::{
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use topcoat_core::{
    context::CxHandle,
    error::{Error, Result},
};

use crate::{View, ViewPart};

type DeferredFuture = Pin<Box<dyn Future<Output = Result<View>> + Send + 'static>>;
type DeferredRender = Box<dyn FnOnce(CxHandle) -> DeferredFuture + Send + 'static>;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct DeferredState {
    render: Option<DeferredRender>,
}

/// Work recorded while rendering a deferred view placeholder.
#[doc(hidden)]
#[derive(Clone)]
pub struct DeferredTask {
    id: u64,
    state: Arc<Mutex<DeferredState>>,
}

impl DeferredTask {
    #[inline]
    #[must_use]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Resolves the deferred view against an owned request context.
    ///
    /// # Errors
    ///
    /// Returns the deferred renderer's error, or an error if the same task is
    /// resolved more than once.
    pub async fn resolve(self, cx: CxHandle) -> Result<View> {
        let render = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .render
            .take()
            .ok_or_else(|| std::io::Error::other("deferred view was already resolved"))?;
        render(cx).await
    }
}

impl fmt::Debug for DeferredTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeferredTask")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// A placeholder and the work that will replace it in a streaming response.
#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct DeferredPart {
    task: DeferredTask,
    placeholder: Box<ViewPart>,
}

impl DeferredPart {
    #[inline]
    #[must_use]
    pub(crate) fn task(&self) -> &DeferredTask {
        &self.task
    }

    #[inline]
    #[must_use]
    pub(crate) fn placeholder(&self) -> &ViewPart {
        &self.placeholder
    }
}

/// Creates a view that renders `placeholder` immediately and streams the
/// completed view later when it is used as an HTTP response.
///
/// Views created this way compose like any other [`View`]. A parent view
/// automatically carries deferred work from every nested child.
#[must_use]
pub fn defer<F, Fut>(placeholder: View, render: F) -> View
where
    F: FnOnce(CxHandle) -> Fut + Send + 'static,
    Fut: Future<Output = Result<View, Error>> + Send + 'static,
{
    let task = DeferredTask {
        id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        state: Arc::new(Mutex::new(DeferredState {
            render: Some(Box::new(move |cx| Box::pin(render(cx)))),
        })),
    };
    View::from_part(ViewPart::Deferred(DeferredPart {
        task,
        placeholder: Box::new(placeholder.into_part()),
    }))
}
