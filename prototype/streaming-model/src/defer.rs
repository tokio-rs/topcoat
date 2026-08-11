use std::cell::OnceCell;
use std::future::Future;
use std::rc::Rc;

use crate::cx::Cx;

/// The state a render observes for one deferred load.
///
/// `Ready` hands out a borrow of the stored output, per the design's implicit
/// memoize leaning: the framework owns the output and every pass reads it in
/// place.
pub enum Deferred<'a, T> {
    Pending,
    Ready(&'a T),
}

/// A handle to one deferred load.
///
/// Created in the body, observed per pass from the render with [`Defer::poll`].
/// The handle lives in the component's frame, so `Ready(&T)` borrows request
/// lived storage through it without any global cache.
pub struct Defer<T> {
    slot: Rc<OnceCell<T>>,
}

impl<T: 'static> Defer<T> {
    /// Registers the future with the request driver and returns the handle.
    ///
    /// The future must be `'static`: it runs between passes, while the
    /// component that registered it is suspended, so it cannot borrow the
    /// component's locals. Data it needs travels owned.
    pub fn spawn(cx: &Cx, name: &'static str, fut: impl Future<Output = T> + 'static) -> Self {
        let slot = Rc::new(OnceCell::new());
        let write = slot.clone();
        let cx2 = cx.clone();
        cx.push_deferred(Box::pin(async move {
            let value = fut.await;
            let _ = write.set(value);
            cx2.log(format!("resolve:{name}"));
        }));
        Defer { slot }
    }

    /// Observes the load for the current pass.
    pub fn poll(&self) -> Deferred<'_, T> {
        match self.slot.get() {
            Some(value) => Deferred::Ready(value),
            None => Deferred::Pending,
        }
    }
}
