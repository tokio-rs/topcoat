use std::borrow::Cow;

use axum::routing::get;

use crate::{Layout, Page, Segment};

#[derive(Default)]
pub struct Router {
    pub(crate) file_root: Option<Cow<'static, str>>,
    pub(crate) pages: Vec<Page>,
    pub(crate) layouts: Vec<Layout>,
    pub(crate) segments: Vec<Segment>,
}

impl Router {
    pub fn new() -> Self {
        Default::default()
    }

    #[doc(hidden)]
    pub fn file_root(mut self, file_root: impl Into<Cow<'static, str>>) -> Self {
        assert!(
            self.segments.is_empty() && self.pages.is_empty() && self.layouts.is_empty(),
            "`file_root` must be called before registering any resource"
        );
        self.file_root = Some(file_root.into());
        self
    }

    #[doc(hidden)]
    pub fn segment(mut self, segment: Segment) -> Self {
        assert!(
            self.file_root.is_some(),
            "segments may only be used as part of a file router"
        );
        self.segments.push(segment);
        self
    }

    pub fn page(mut self, page: Page) -> Self {
        assert!(
            page.path().is_some() || self.file_root.is_some(),
            "page is missing a path, which is only allowed in file router"
        );
        self.pages.push(page);
        self
    }

    pub fn layout(mut self, layout: Layout) -> Self {
        assert!(
            layout.path().is_some() || self.file_root.is_some(),
            "layout is missing a path, which is only allowed in file router"
        );
        self.layouts.push(layout);
        self
    }

    #[cfg(feature = "discover")]
    pub fn discover(mut self) -> Self {
        if self.file_root.is_some() {
            for segment in inventory::iter::<Segment>().cloned() {
                self = self.segment(segment);
            }
        }

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
            let path = page.path().map_or_else(
                || Cow::Owned(value.path_from_file(page.file())),
                Cow::Borrowed,
            );

            result = result.route(
                &path.to_axum_path(),
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
