use std::collections::HashMap;

use http::Method;
use matchit::Params;

use crate::{Path, Pattern, pattern::Segment, route::RouteId};

#[derive(Debug, Default, Clone)]
pub(crate) struct DynamicRoutes {
    routers: HashMap<Method, matchit::Router<RouteId>>,
}

impl DynamicRoutes {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert(&mut self, method: Method, pattern: &Pattern, route_id: RouteId) {
        self.routers
            .entry(method)
            .or_default()
            .insert(convert_pattern(pattern), route_id)
            .unwrap();
    }

    pub fn get<'path>(
        &self,
        method: &Method,
        path: &'path Path<'_>,
    ) -> Option<DynamicMatch<'_, 'path>> {
        self.routers
            .get(method)?
            .at(path)
            .ok()
            .map(|result| DynamicMatch::new(*result.value, result.params))
    }
}

fn convert_pattern(pattern: &Pattern) -> String {
    let mut out = String::new();
    for segment in pattern.segments() {
        out.push('/');
        match segment {
            Segment::Static(s) => {
                for ch in s.chars() {
                    match ch {
                        '{' => out.push_str("{{"),
                        '}' => out.push_str("}}"),
                        _ => out.push(ch),
                    }
                }
            }
            Segment::Dynamic(name) => {
                out.push('{');
                out.push_str(name);
                out.push('}');
            }
            Segment::CatchAll(name) => {
                out.push_str("{*");
                out.push_str(name);
                out.push('}');
            }
        }
    }
    out
}

pub(crate) struct DynamicMatch<'k, 'v> {
    pub(crate) route_id: RouteId,
    pub(crate) params: Params<'k, 'v>,
}

impl<'k, 'v> DynamicMatch<'k, 'v> {
    fn new(route_id: RouteId, params: matchit::Params<'k, 'v>) -> Self {
        Self { route_id, params }
    }
}

#[cfg(test)]
mod tests {
    use crate::TryIntoPattern;

    use super::*;

    #[test]
    fn convert_simple_dynamic() {
        let pattern = "/users/:id".try_into_pattern().unwrap();
        assert_eq!(convert_pattern(&pattern), "/users/{id}");
    }

    #[test]
    fn convert_static_with_braces() {
        let pattern = "/users/{literal}".try_into_pattern().unwrap();
        assert_eq!(convert_pattern(&pattern), "/users/{{literal}}");
    }

    #[test]
    fn convert_mixed() {
        let pattern = "/users/:id/posts".try_into_pattern().unwrap();
        assert_eq!(convert_pattern(&pattern), "/users/{id}/posts");
    }
}
