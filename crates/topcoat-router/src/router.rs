use std::borrow::Cow;

use axum::routing::get;

use crate::{layout::Layout, page::Page};

#[derive(Default)]
pub struct Router {
    pub(crate) file_root: Option<Cow<'static, str>>,
    pub(crate) pages: Vec<Page>,
    pub(crate) layouts: Vec<Layout>,
}

impl Router {
    pub fn new() -> Self {
        Default::default()
    }

    #[doc(hidden)]
    pub fn file_root(mut self, file_root: impl Into<Cow<'static, str>>) -> Self {
        if !self.pages.is_empty() || !self.layouts.is_empty() {
            panic!("`file_root` must be called before registering any pages or layouts");
        }
        self.file_root = Some(file_root.into());
        self
    }

    pub fn page(mut self, page: Page) -> Self {
        if page.path().is_none() && self.file_root.is_none() {
            panic!("page is missing a path, which is only allowed in file router")
        }
        self.pages.push(page);
        self
    }

    pub fn layout(mut self, layout: Layout) -> Self {
        if layout.path().is_none() && self.file_root.is_none() {
            panic!("layout is missing a path, which is only allowed in file router")
        }
        self.layouts.push(layout);
        self
    }

    #[cfg(feature = "discover")]
    pub fn discover(mut self) -> Self {
        for page in inventory::iter::<Page>().cloned() {
            self = self.page(page);
        }
        for layout in inventory::iter::<Layout>().cloned() {
            self = self.layout(layout);
        }
        self
    }
}

impl From<Router> for axum::Router {
    fn from(value: Router) -> Self {
        let mut result = axum::Router::new();

        for page in &value.pages {
            let page = page.clone();
            let layouts = value.layouts.clone();
            let path = page
                .path()
                .map(Cow::Borrowed)
                .unwrap_or_else(|| Cow::Owned(value.path_from_file(page.file())));

            result = result.route(
                &path,
                get(async move || {
                    let mut result = page.render();
                    for layout in layouts {
                        result = layout.render(result);
                    }
                    result.await
                }),
            );
        }

        result
    }
}
