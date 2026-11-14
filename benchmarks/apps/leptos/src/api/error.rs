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
        match error {
            ListingError::CategoryNotFound => Self::not_found("category not found"),
            ListingError::PageNotFound => Self::not_found("page not found"),
            ListingError::InvalidSort => Self::invalid("invalid sort"),
            ListingError::InvalidPage => Self::invalid("invalid page"),
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
