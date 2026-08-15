use crate::RouteId;

pub trait HrefTarget {
    fn route_id(&self) -> RouteId;
}
