use std::{convert::Infallible, sync::Arc};

use crate::Result;
use axum::extract::{FromRequest, FromRequestParts, RawPathParams};
use topcoat_core::context::{Cx, State};

pub type Body = axum::body::Body;

pub(crate) struct CxBody {
    pub(crate) cx: Cx,
    pub(crate) body: Body,
}

impl FromRequest<Arc<State>> for CxBody {
    type Rejection = Infallible;

    async fn from_request(
        req: axum::extract::Request,
        state: &Arc<State>,
    ) -> Result<Self, Self::Rejection> {
        let app_state = state.clone();
        let (mut parts, body) = req.into_parts();
        let body = Body::from(body);

        let mut request_state = State::new();
        request_state.register(RawPathParams::from_request_parts(&mut parts, state).await);
        request_state.register(parts);

        let cx = Cx::new(app_state, request_state);
        Ok(Self { cx, body })
    }
}

pub trait FromBody: Sized {
    fn from_body(cx: &Cx, body: Body) -> impl Future<Output = Result<Self>> + Send;
}

impl FromBody for Body {
    async fn from_body(_cx: &Cx, body: Body) -> Result<Self> {
        Ok(body)
    }
}
