pub mod layout;
pub mod page;

use axum::routing::get;

use crate::{layout::Layout, page::Page};

#[derive(Default)]
pub struct Router {
    pages: Vec<Page>,
    layouts: Vec<Layout>,
}

impl Router {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn page(mut self, page: Page) -> Self {
        self.pages.push(page);
        self
    }

    pub fn layout(mut self, layout: Layout) -> Self {
        self.layouts.push(layout);
        self
    }
}

impl From<Router> for axum::Router {
    fn from(value: Router) -> Self {
        let mut result = axum::Router::new();

        for page in value.pages {
            let layouts = value.layouts.clone();
            result = result.route(
                page.path(),
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
