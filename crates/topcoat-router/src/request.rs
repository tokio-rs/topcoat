use std::sync::Arc;

use axum::extract::{FromRequestParts, RawPathParams};
use topcoat_core::{
    context::{Cx, State},
    error::Result,
};

use crate::error::{BadRequestError, bad_request};

pub type Body = axum::body::Body;

pub(crate) struct CxBody {
    pub(crate) cx: Cx,
    pub(crate) body: Body,
}

impl axum::extract::FromRequest<Arc<State>> for CxBody {
    type Rejection = BadRequestError;

    async fn from_request(
        req: axum::extract::Request,
        state: &Arc<State>,
    ) -> Result<Self, Self::Rejection> {
        let app_state = state.clone();
        let (mut parts, body) = req.into_parts();
        let body = Body::from(body);

        let mut request_state = State::new();
        request_state.register(
            RawPathParams::from_request_parts(&mut parts, state)
                .await
                .map_err(|error| bad_request(error.to_string()))?,
        );
        request_state.register(parts);

        let cx = Cx::new(app_state, request_state);
        Ok(Self { cx, body })
    }
}

pub trait FromRequest: Sized {
    fn from_request(cx: &Cx, body: Body) -> impl Future<Output = Result<Self>> + Send;
}

impl FromRequest for Body {
    async fn from_request(_cx: &Cx, body: Body) -> Result<Self> {
        Ok(body)
    }
}
