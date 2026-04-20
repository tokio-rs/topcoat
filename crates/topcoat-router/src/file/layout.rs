use std::{borrow::Cow, pin::Pin};

use topcoat_view::runtime::View;

use crate::{Layout, Path, Slot};

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct FileLayout {
    file: &'static str,
    render: fn(slot: Slot) -> Pin<Box<dyn Future<Output = View> + Send>>,
}

impl FileLayout {
    pub const fn new(
        file: &'static str,
        render: fn(slot: Slot) -> Pin<Box<dyn Future<Output = View> + Send>>,
    ) -> Self {
        Self { file, render }
    }

    pub fn into_layout(self, path: Cow<'static, Path>) -> Layout {
        Layout::new(path, self.render)
    }

    pub fn file(&self) -> &'static str {
        self.file
    }
}

#[cfg(feature = "discover")]
inventory::collect!(FileLayout);
