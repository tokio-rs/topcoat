//! The JSON API.

pub mod cart;
pub mod products;

use std::{borrow::Cow, fmt};

use serde::Serialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, Next, StatusCode,
        content::Json,
        layer,
        response::{IntoResponse, Response},
    },
};

/// An API error, answered with an `{"error": "..."}` body.
#[derive(Debug, Clone)]
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

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        #[derive(Serialize)]
        struct ErrorBody<'a> {
            error: &'a str,
        }

        let body = ErrorBody {
            error: &self.message,
        };
        (self.status, Json(body)).into_response(cx)
    }
}

/// Answers the [`ApiError`]s of API routes with their JSON body.
#[layer]
async fn api_errors(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let result = next.run(cx, body).await;
    if let Err(error) = &result
        && let Some(api_error) = error.downcast_ref::<ApiError>()
    {
        return api_error.clone().into_response(cx);
    }
    result
}
