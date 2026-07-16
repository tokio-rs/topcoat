use topcoat_router::RouterBuilder;

use crate::Config;

pub trait RouterBuilderCookieExt {
    #[must_use]
    fn sessions(self, config: Config) -> Self;
}

impl RouterBuilderCookieExt for RouterBuilder {
    fn sessions(mut self, config: Config) -> Self {
        self = self.app_context(config);
        self
    }
}
