use topcoat_core::context::Cx;

use crate::runtime::{Formatter, Fragment};

pub type Child = dyn AsyncFn(&Cx, &mut Formatter<'_>);

impl Fragment for Child {
    fn fmt(&self, cx: &Cx, f: &mut Formatter<'_>) {
        self(cx, f)
    }
}
