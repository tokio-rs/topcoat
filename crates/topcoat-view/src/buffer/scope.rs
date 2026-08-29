use std::cell::Cell;

use crate::buffer::ViewBuffer;

thread_local! {
    static CURRENT: Cell<Option<Box<ViewBuffer>>> = const { Cell::new(None) };
}

pub(crate) struct ViewBufferScope<'a> {
    slot: &'a mut Option<Box<ViewBuffer>>,
}

impl<'a> ViewBufferScope<'a> {
    pub(crate) fn new(slot: &'a mut Option<Box<ViewBuffer>>) -> Self {
        *slot = CURRENT.replace(slot.take());
        Self { slot }
    }

    pub(crate) fn is_active() -> bool {
        let buffer = CURRENT.take();
        let active = buffer.is_some();
        CURRENT.set(buffer);
        active
    }

    pub(crate) fn with<R>(f: impl FnOnce(&mut ViewBuffer) -> R) -> R {
        let mut buffer = CURRENT.take();
        let result = f(buffer
            .as_mut()
            .expect("tried to access the scope's ViewBuffer outside of a `ViewBufferScope`"));
        CURRENT.set(buffer);
        result
    }
}

impl Drop for ViewBufferScope<'_> {
    fn drop(&mut self) {
        *self.slot = CURRENT.replace(self.slot.take());
    }
}
