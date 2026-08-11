use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

/// Creates a manually fired one-shot future and its firing handle.
///
/// Tests use triggers to stand in for I/O: a component awaits the [`Trigger`]
/// and the test decides exactly when it completes by calling [`Fire::fire`].
/// This keeps every scenario deterministic without an executor or clock.
pub fn trigger() -> (Trigger, Fire) {
    let fired = Rc::new(Cell::new(false));
    (
        Trigger {
            fired: fired.clone(),
        },
        Fire { fired },
    )
}

/// A future that completes once its paired [`Fire`] handle has fired.
pub struct Trigger {
    fired: Rc<Cell<bool>>,
}

impl Future for Trigger {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _ctx: &mut Context<'_>) -> Poll<()> {
        if self.fired.get() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

/// The test-side handle that completes a [`Trigger`].
pub struct Fire {
    fired: Rc<Cell<bool>>,
}

impl Fire {
    pub fn fire(&self) {
        self.fired.set(true);
    }
}
