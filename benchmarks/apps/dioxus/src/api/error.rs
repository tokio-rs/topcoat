use std::borrow::Cow;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::store::ListingError;

/// A JSON error response with an `{"error": "..."}` body.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: Cow<'static, str>,
}

impl ApiError {
    pub fn not_found(message: &'static str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: message.into(),
        }
    }
}

impl From<ListingError> for ApiError {
    fn from(error: ListingError) -> Self {
        Self {
            status: error.status(),
            message: error.message().into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct Body<'a> {
            error: &'a str,
        }

        (
            self.status,
            Json(Body {
                error: &self.message,
            }),
        )
            .into_response()
    }
}
