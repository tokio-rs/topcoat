use std::{borrow::Cow, pin::Pin};

use topcoat_view::runtime::View;

use crate::{Page, Path};

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct FilePage {
    file: &'static str,
    pub(super) render: fn() -> Pin<Box<dyn Future<Output = View> + Send>>,
}

impl FilePage {
    pub const fn new(
        file: &'static str,
        render: fn() -> Pin<Box<dyn Future<Output = View> + Send>>,
    ) -> Self {
        Self { file, render }
    }

    pub fn into_page(self, path: Cow<'static, Path>) -> Page {
        Page::new(path, self.render)
    }

    pub fn file(&self) -> &'static str {
        self.file
    }
}

#[cfg(feature = "discover")]
inventory::collect!(FilePage);
