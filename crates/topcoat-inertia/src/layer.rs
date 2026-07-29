use std::sync::Arc;

use http::header::{CONTENT_LENGTH, CONTENT_TYPE, LOCATION, REFERER};
use http::{HeaderName, HeaderValue, Method, StatusCode, Uri};
use http_body::Body as _;
use topcoat_core::base_url::try_base_url;
use topcoat_core::context::{Cx, CxBuilder};
use topcoat_router::error::RedirectError;
use topcoat_router::{
    Body, IntoResponse, Layer, LayerFuture, Next, Path, Response, RouterBuilder, parts,
};

use crate::flash::{FlashPayload, FlashState, IncludedIncomingFlash};
use crate::response::add_vary;
use crate::{InertiaConfig, InertiaRequest, SharedProps, header};

/// The root router layer that implements Inertia.js v3 visit behavior.
pub struct InertiaLayer {
    config: Arc<InertiaConfig>,
}

impl InertiaLayer {
    /// Creates an Inertia layer using `config`.
    #[must_use]
    pub fn new(config: Arc<InertiaConfig>) -> Self {
        Self { config }
    }
}

impl Layer for InertiaLayer {
    fn path(&self) -> &Path {
        Path::new("/")
    }

    fn handle<'a>(&'a self, cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            let request = InertiaRequest::from_headers(&parts(cx).headers);
            let incoming = self
                .config
                .flash_store
                .read(cx)
                .await?
                .map(|payload| serde_json::from_slice::<FlashPayload>(&payload))
                .transpose()?
                .unwrap_or_default();
            cx.insert(request);
            cx.insert(SharedProps::default());
            cx.insert(FlashState::new(incoming));

            let mut response = match next.run(cx, body).await {
                Ok(response) => response,
                Err(error) => match error.downcast::<RedirectError>() {
                    Ok(redirect) => redirect.into_response(cx)?,
                    Err(error) => return Err(error),
                },
            };
            let request = cx
                .get::<InertiaRequest>()
                .expect("the Inertia layer inserted request state");
            let request_parts = parts(cx);
            let original_redirect = is_redirect(response.status());
            let mut preserve_flash = original_redirect;

            let server_version = self.config.version.as_deref().unwrap_or("");
            let client_version = request.version().unwrap_or("");
            if request.is_inertia()
                && request_parts.method == Method::GET
                && server_version != client_version
                && !response.headers().contains_key(&header::X_INERTIA_LOCATION)
            {
                let target = version_location(cx);
                transform_protocol_response(
                    &mut response,
                    &header::X_INERTIA_LOCATION,
                    HeaderValue::from_str(&target)?,
                );
                response.headers_mut().insert(
                    &header::X_INERTIA_VERSION,
                    HeaderValue::from_str(server_version)?,
                );
                preserve_flash = true;
            } else if request.is_inertia()
                && response.status() == StatusCode::OK
                && response.body().is_end_stream()
            {
                let target = redirect_back_target(cx);
                transform_protocol_response(
                    &mut response,
                    &header::X_INERTIA_LOCATION,
                    HeaderValue::from_str(&target)?,
                );
                preserve_flash = true;
            } else {
                normalize_redirect_method(&mut response, request_parts);

                if request.is_inertia()
                    && !request.is_prefetch()
                    && matches!(
                        response.status(),
                        StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND | StatusCode::SEE_OTHER
                    )
                    && location(&response).is_some_and(|location| location.contains('#'))
                {
                    let target = location(&response)
                        .expect("the fragment location was just checked")
                        .to_owned();
                    transform_protocol_response(
                        &mut response,
                        &header::X_INERTIA_REDIRECT,
                        HeaderValue::from_str(&target)?,
                    );
                    preserve_flash = true;
                } else if request.is_inertia()
                    && self.config.convert_external_redirects
                    && matches!(
                        response.status(),
                        StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND | StatusCode::SEE_OTHER
                    )
                    && location(&response).is_some_and(|location| is_external(cx, location))
                {
                    let target = location(&response)
                        .expect("the external location was just checked")
                        .to_owned();
                    transform_protocol_response(
                        &mut response,
                        &header::X_INERTIA_LOCATION,
                        HeaderValue::from_str(&target)?,
                    );
                    preserve_flash = true;
                }
            }

            let flash = cx
                .get::<FlashState>()
                .expect("the Inertia layer inserted flash state");
            if preserve_flash {
                let payload = flash.combined();
                if !payload.is_empty() {
                    let payload = serde_json::to_vec(&payload)?;
                    self.config.flash_store.write(cx, &payload).await?;
                }
            } else if response
                .extensions()
                .get::<IncludedIncomingFlash>()
                .is_some()
            {
                self.config.flash_store.delete(cx).await?;
            }

            Ok(response)
        })
    }
}

/// Adds Inertia.js v3 protocol handling to a router builder.
pub trait RouterBuilderInertiaExt {
    /// Registers the Inertia configuration and root protocol layer.
    ///
    /// Register `.cookies()` after this extension so the cookie layer wraps
    /// Inertia and can emit changes made by the default flash store.
    #[must_use]
    fn inertia(self, config: InertiaConfig) -> Self;
}

impl RouterBuilderInertiaExt for RouterBuilder {
    fn inertia(self, config: InertiaConfig) -> Self {
        let config = Arc::new(config);
        self.app_context(config.clone())
            .layer(InertiaLayer::new(config))
    }
}

fn is_redirect(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::MOVED_PERMANENTLY
            | StatusCode::FOUND
            | StatusCode::SEE_OTHER
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::PERMANENT_REDIRECT
    )
}

fn normalize_redirect_method(response: &mut Response, request: &http::request::Parts) {
    if matches!(request.method, Method::PUT | Method::PATCH | Method::DELETE)
        && matches!(
            response.status(),
            StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND
        )
    {
        *response.status_mut() = StatusCode::SEE_OTHER;
    }
}

fn transform_protocol_response(response: &mut Response, name: &HeaderName, value: HeaderValue) {
    *response.status_mut() = StatusCode::CONFLICT;
    *response.body_mut() = Body::empty();
    response.headers_mut().remove(LOCATION);
    response.headers_mut().remove(CONTENT_TYPE);
    response.headers_mut().remove(CONTENT_LENGTH);
    response.headers_mut().remove(&header::X_INERTIA);
    response.headers_mut().remove(&header::X_INERTIA_LOCATION);
    response.headers_mut().remove(&header::X_INERTIA_REDIRECT);
    response.headers_mut().insert(name, value);
    add_vary(response.headers_mut());
}

fn location(response: &Response) -> Option<&str> {
    response.headers().get(LOCATION)?.to_str().ok()
}

fn request_target(cx: &Cx) -> &str {
    parts(cx)
        .uri
        .path_and_query()
        .map_or("/", http::uri::PathAndQuery::as_str)
}

fn version_location(cx: &Cx) -> String {
    try_base_url(cx).map_or_else(
        || request_target(cx).to_owned(),
        |base| base.join(request_target(cx)),
    )
}

fn redirect_back_target(cx: &Cx) -> String {
    parts(cx)
        .headers
        .get(REFERER)
        .and_then(|referer| referer.to_str().ok())
        .filter(|referer| safe_referer(cx, referer))
        .map_or_else(|| request_target(cx).to_owned(), str::to_owned)
}

fn safe_referer(cx: &Cx, referer: &str) -> bool {
    if referer.starts_with('/') {
        return !referer.starts_with("//") && referer.parse::<Uri>().is_ok();
    }
    let Ok(referer) = referer.parse::<Uri>() else {
        return false;
    };
    let Some(base) = try_base_url(cx) else {
        return false;
    };
    let Ok(base) = base.as_str().parse::<Uri>() else {
        return false;
    };
    same_origin(&referer, &base)
}

fn is_external(cx: &Cx, location: &str) -> bool {
    let Ok(location) = location.parse::<Uri>() else {
        return false;
    };
    if location.scheme().is_none() || location.authority().is_none() {
        return false;
    }
    let Some(base) = try_base_url(cx) else {
        return true;
    };
    let Ok(base) = base.as_str().parse::<Uri>() else {
        return true;
    };
    !same_origin(&location, &base)
}

fn same_origin(left: &Uri, right: &Uri) -> bool {
    left.scheme_str()
        .zip(right.scheme_str())
        .is_some_and(|(left, right)| left.eq_ignore_ascii_case(right))
        && left.authority() == right.authority()
}
