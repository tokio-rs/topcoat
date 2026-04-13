use std::pin::Pin;

use topcoat_view::runtime::View;

pub type Slot = Pin<Box<dyn Future<Output = View> + Send>>;

#[derive(Clone)]
pub struct Layout {
    file: &'static str,
    path: &'static str,
    render: fn(slot: Slot) -> Pin<Box<dyn Future<Output = View> + Send>>,
}

impl Layout {
    pub const fn new(
        file: &'static str,
        path: &'static str,
        render: fn(slot: Slot) -> Pin<Box<dyn Future<Output = View> + Send>>,
    ) -> Self {
        Self { file, path, render }
    }

    pub fn file(&self) -> &'static str {
        self.file
    }

    pub fn path(&self) -> &'static str {
        self.path
    }

    pub fn render(&self, slot: Slot) -> Pin<Box<dyn Future<Output = View> + Send>> {
        (self.render)(slot)
    }
}
