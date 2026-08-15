use topcoat_core::context::Cx;

use crate::Path;

pub trait HrefTarget {
    fn path<'cx>(&self, cx: &'cx Cx) -> &'cx Path;
}
