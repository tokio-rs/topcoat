use std::collections::HashMap;

use http::Method;

use crate::{Path, route::RouteId};

#[derive(Debug, Default, Clone)]
pub(crate) struct StaticRoutes {
    routes: HashMap<StaticRouteKey<'static>, RouteId>,
}

impl StaticRoutes {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert(&mut self, method: Method, path: Path<'static>, route_id: RouteId) {
        self.routes
            .insert(StaticRouteKey::new(method, path), route_id);
    }

    pub fn get(&self, method: &Method, path: &Path<'_>) -> Option<RouteId> {
        self.routes
            .get(&StaticRouteKey::new(method.clone(), path.clone()))
            .copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct StaticRouteKey<'a> {
    method: Method,
    path: Path<'a>,
}

impl<'a> StaticRouteKey<'a> {
    fn new(method: Method, path: Path<'a>) -> Self {
        Self { method, path }
    }
}
