use http::Method;

use crate::{
    Handler, Params, Path, Pattern, dynamic_routes::DynamicRoutes, static_routes::StaticRoutes,
};

#[derive(Debug, Clone)]
pub struct Route {
    method: Method,
    pattern: Pattern,
    handler: HandlerFn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RouteId(usize);

impl RouteId {
    fn new(inner: usize) -> Self {
        Self(inner)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Routes {
    routes: Vec<Route>,
    static_routes: StaticRoutes,
    dynamic_routes: DynamicRoutes,
}

impl Routes {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert(&mut self, route: Route) {
        let route_id = RouteId::new(self.routes.len());
        if let Some(path) = route.pattern.to_path() {
            self.static_routes
                .insert(route.method.clone(), path, route_id);
        } else {
            self.dynamic_routes
                .insert(route.method.clone(), &route.pattern, route_id);
        }
        self.routes.push(route);
    }

    pub fn get<'path>(&self, method: &Method, path: &'path Path<'_>) -> Option<Match<'_, 'path>> {
        if let Some(static_route) = self.static_routes.get(method, path) {
            Some(Match {
                route: &self.routes[static_route.0],
                params: Default::default(),
            })
        } else if let Some(dynamic_match) = self.dynamic_routes.get(method, path) {
            Some(Match {
                route: &self.routes[dynamic_match.route_id.0],
                params: dynamic_match.params,
            })
        } else {
            None
        }
    }
}

pub struct Match<'k, 'v> {
    route: &'k Route,
    params: Params<'k, 'v>,
}

impl<'k, 'v> Match<'k, 'v> {
    pub fn handle(&self) {
        self.route.handler.handle();
    }
}
