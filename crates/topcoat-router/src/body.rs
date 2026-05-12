use crate::Result;
use topcoat_core::context::Cx;

pub type Body = axum::body::Body;

pub trait FromBody: Sized {
    fn from_body(cx: &Cx, body: Body) -> impl Future<Output = Result<Self>> + Send;
}
