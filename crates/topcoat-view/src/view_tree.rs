use core::{future::Future, pin::Pin};
use std::sync::{Arc, Mutex};

use futures_util::future::join_all;
use topcoat_core::error::{Error, Result};

use crate::{View, ViewPart, ViewParts};

type ViewFuture<'a> = Pin<Box<dyn Future<Output = Result<View>> + Send + 'a>>;

enum ViewTreeNode<'a> {
    Ready(View),
    Pending(ViewFuture<'a>),
}

/// A view placeholder filled by generated component code before rendering.
#[doc(hidden)]
#[derive(Clone, Debug, Default)]
pub struct ViewSlot {
    part: Arc<Mutex<Option<ViewPart>>>,
}

impl ViewSlot {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    #[must_use]
    pub fn view(&self) -> View {
        let mut parts = ViewParts::new();
        parts.push_part(ViewPart::Slot(Arc::clone(&self.part)));
        View::new(parts)
    }

    /// Fills the placeholder with its completed child view.
    ///
    /// # Panics
    ///
    /// Panics if the slot was already filled or its lock was poisoned.
    #[inline]
    pub fn fill(&self, view: View) {
        let previous = self.part.lock().unwrap().replace(view.into_part());
        assert!(previous.is_none(), "view slot must only be filled once");
    }
}

/// Collects ready view segments and component futures before resolving them.
///
/// This is plumbing for generated `view!` code.
#[doc(hidden)]
#[derive(Default)]
pub struct ViewTree<'a> {
    nodes: Vec<ViewTreeNode<'a>>,
}

impl<'a> ViewTree<'a> {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn push_view(&mut self, view: View) {
        self.nodes.push(ViewTreeNode::Ready(view));
    }

    #[inline]
    pub fn push_future(&mut self, future: impl Future<Output = Result<View, Error>> + Send + 'a) {
        self.nodes.push(ViewTreeNode::Pending(Box::pin(future)));
    }

    /// Resolves pending nodes and combines every view in tree order.
    ///
    /// # Errors
    ///
    /// Returns the first component error in tree order after every pending
    /// node completes.
    pub async fn resolve(self) -> Result<View> {
        let views = join_all(self.nodes.into_iter().map(|node| async move {
            match node {
                ViewTreeNode::Ready(view) => Ok(view),
                ViewTreeNode::Pending(future) => future.await,
            }
        }))
        .await;

        let mut parts = ViewParts::new();
        for view in views {
            parts.push_view(view?);
        }
        Ok(View::new(parts))
    }
}
