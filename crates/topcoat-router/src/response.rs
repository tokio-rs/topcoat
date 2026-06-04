use crate::Body;
use topcoat_core::error::Result;

pub type Response<T = Body> = http::Response<T>;

pub trait IntoResponse {
    fn into_response(self) -> Result<Response>;
}

impl<T> IntoResponse for T
where
    T: axum::response::IntoResponse,
{
    fn into_response(self) -> Result<Response> {
        Ok(axum::response::IntoResponse::into_response(self))
    }
}
