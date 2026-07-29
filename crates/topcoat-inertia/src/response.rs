use std::fmt;
use std::sync::Arc;

use http::header::{CONTENT_TYPE, HeaderValue, LOCATION, VARY};
use http::{HeaderMap, StatusCode};
use serde::Serialize;
use serde_json::{Map, Value};
use topcoat_core::context::{Cx, try_app_context, try_request_context};
use topcoat_core::error::Result;
use topcoat_router::{Body, IntoResponse, Response, uri};

use crate::flash::{FlashState, IncludedIncomingFlash};
use crate::resolver::{PropEntry, resolve};
use crate::{
    InertiaConfig, InertiaRequest, Page, Prop, Props, ScrollMetadata, SharedProps, always, defer,
    header, lazy, merge, once, optional, scroll,
};

pub struct Inertia<'cx> {
    component: String,
    props: Vec<(String, Prop<'cx>)>,
    errors: Option<Result<Value>>,
    flash: Vec<(String, Result<Value>)>,
    url: Option<String>,
    encrypt_history: Option<bool>,
    clear_history: bool,
    preserve_fragment: bool,
}

impl Inertia<'_> {
    #[must_use]
    pub fn new(component: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            props: Vec::new(),
            errors: None,
            flash: Vec::new(),
            url: None,
            encrypt_history: None,
            clear_history: false,
            preserve_fragment: false,
        }
    }
}

impl<'cx> Inertia<'cx> {
    #[must_use]
    pub fn errors(mut self, errors: impl Serialize) -> Self {
        self.errors = Some(serde_json::to_value(errors).map_err(Into::into));
        self
    }

    #[must_use]
    pub fn flash(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        self.flash
            .push((key.into(), serde_json::to_value(value).map_err(Into::into)));
        self
    }

    #[must_use]
    pub fn prop(self, key: impl Into<String>, value: impl Serialize) -> Self {
        self.prop_with(key, Prop::value(value))
    }

    #[must_use]
    pub fn lazy<T>(
        self,
        key: impl Into<String>,
        future: impl Future<Output = Result<T>> + Send + 'cx,
    ) -> Self
    where
        T: Serialize,
    {
        self.prop_with(key, lazy(future))
    }

    #[must_use]
    pub fn always(self, key: impl Into<String>, value: impl Serialize) -> Self {
        self.prop_with(key, always(value))
    }

    #[must_use]
    pub fn optional<T>(
        self,
        key: impl Into<String>,
        future: impl Future<Output = Result<T>> + Send + 'cx,
    ) -> Self
    where
        T: Serialize,
    {
        self.prop_with(key, optional(future))
    }

    #[must_use]
    pub fn defer<T>(
        self,
        key: impl Into<String>,
        future: impl Future<Output = Result<T>> + Send + 'cx,
    ) -> Self
    where
        T: Serialize,
    {
        self.prop_with(key, defer(future))
    }

    #[must_use]
    pub fn merge(self, key: impl Into<String>, value: impl Serialize) -> Self {
        self.prop_with(key, merge(value))
    }

    #[must_use]
    pub fn once<T>(
        self,
        key: impl Into<String>,
        future: impl Future<Output = Result<T>> + Send + 'cx,
    ) -> Self
    where
        T: Serialize,
    {
        self.prop_with(key, once(future))
    }

    #[must_use]
    pub fn scroll(
        self,
        key: impl Into<String>,
        value: impl Serialize,
        metadata: ScrollMetadata,
    ) -> Self {
        self.prop_with(key, scroll(value, metadata))
    }

    #[must_use]
    pub fn prop_with(mut self, key: impl Into<String>, prop: Prop<'cx>) -> Self {
        self.props.push((key.into(), prop));
        self
    }

    #[must_use]
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    #[must_use]
    pub fn encrypt_history(mut self, encrypt: bool) -> Self {
        self.encrypt_history = Some(encrypt);
        self
    }

    #[must_use]
    pub fn clear_history(mut self) -> Self {
        self.clear_history = true;
        self
    }

    #[must_use]
    pub fn preserve_fragment(mut self) -> Self {
        self.preserve_fragment = true;
        self
    }

    /// Resolves the selected props and builds an inert page response.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid prop combinations or paths, failed prop
    /// futures, serialization failures, or missing Inertia configuration.
    pub async fn render(self, cx: &'cx Cx) -> Result<InertiaResponse> {
        let config = try_app_context::<Arc<InertiaConfig>>(cx).ok_or_else(|| {
            ResponseError::new("Inertia responses require `.inertia(...)` on the router")
        })?;
        let request = try_request_context::<InertiaRequest>(cx)
            .cloned()
            .unwrap_or_default();

        let mut entries = Vec::new();
        let mut configured = Props::new();
        for share in &config.shared {
            share.share(cx, &mut configured)?;
        }
        entries.extend(
            configured
                .entries
                .into_iter()
                .map(|(path, prop)| PropEntry {
                    path,
                    prop,
                    shared: true,
                }),
        );
        if let Some(shared) = try_request_context::<SharedProps>(cx) {
            entries.extend(shared.take().into_iter().map(|(path, prop)| PropEntry {
                path,
                prop,
                shared: true,
            }));
        }
        entries.extend(self.props.into_iter().map(|(path, prop)| PropEntry {
            path,
            prop,
            shared: false,
        }));
        if entries.iter().any(|entry| entry.path == "errors") {
            return Err(ResponseError::new(
                "`errors` is reserved; use `Inertia::errors(...)` or `flash_errors(...)`",
            )
            .into());
        }

        let incoming = try_request_context::<FlashState>(cx)
            .map(FlashState::incoming)
            .unwrap_or_default();
        let included_incoming = !incoming.is_empty();
        let has_immediate_errors = self.errors.is_some();
        let errors = match self.errors {
            Some(errors) => errors?,
            None => incoming
                .errors
                .clone()
                .unwrap_or_else(|| Value::Object(Map::new())),
        };
        if !errors.is_object() {
            return Err(ResponseError::new("Inertia errors must serialize as an object").into());
        }
        let errors = if has_immediate_errors {
            errors
        } else {
            match (request.error_bag(), errors) {
                (Some(bag), Value::Object(errors)) if !errors.is_empty() => {
                    let mut bags = Map::new();
                    bags.insert(bag.to_owned(), Value::Object(errors));
                    Value::Object(bags)
                }
                (_, errors) => errors,
            }
        };

        let url = self.url.unwrap_or_else(|| uri(cx).to_string());
        let mut page = resolve(
            self.component,
            url,
            config.version.clone(),
            entries,
            errors,
            &request,
        )
        .await?;
        page.encrypt_history = self.encrypt_history.unwrap_or(config.encrypt_history);
        page.clear_history = self.clear_history || incoming.clear_history;
        page.preserve_fragment = self.preserve_fragment || incoming.preserve_fragment;
        page.flash = incoming.data;
        for (key, value) in self.flash {
            page.flash.insert(key, value?);
        }

        Ok(InertiaResponse {
            page,
            included_incoming,
        })
    }
}

#[derive(Debug, Clone)]
#[must_use]
pub struct InertiaResponse {
    page: Page,
    included_incoming: bool,
}

impl InertiaResponse {
    #[must_use]
    pub fn page(&self) -> &Page {
        &self.page
    }
}

impl IntoResponse for InertiaResponse {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        let mut response = if crate::inertia_request(cx) {
            let mut response = Response::new(Body::from(serde_json::to_vec(&self.page)?));
            response.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            );
            response
                .headers_mut()
                .insert(&header::X_INERTIA, HeaderValue::from_static("true"));
            response
        } else {
            let config = try_app_context::<Arc<InertiaConfig>>(cx).ok_or_else(|| {
                ResponseError::new("Inertia responses require `.inertia(...)` on the router")
            })?;
            (config.root)(cx, &self.page).into_response(cx)?
        };
        add_vary(response.headers_mut());
        if self.included_incoming {
            response.extensions_mut().insert(IncludedIncomingFlash);
        }
        Ok(response)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct InertiaLocation {
    url: String,
}

impl InertiaLocation {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

impl IntoResponse for InertiaLocation {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        let value = HeaderValue::from_str(&self.url)?;
        if crate::inertia_request(cx) {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::CONFLICT;
            response
                .headers_mut()
                .insert(&header::X_INERTIA_LOCATION, value);
            add_vary(response.headers_mut());
            Ok(response)
        } else {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::FOUND;
            response.headers_mut().insert(LOCATION, value);
            Ok(response)
        }
    }
}

pub fn inertia_location(url: impl Into<String>) -> InertiaLocation {
    InertiaLocation::new(url)
}

pub(crate) fn add_vary(headers: &mut HeaderMap) {
    let present = headers.get_all(VARY).iter().any(|value| {
        value.to_str().is_ok_and(|value| {
            value
                .split(',')
                .any(|name| name.trim().eq_ignore_ascii_case("X-Inertia"))
        })
    });
    if !present {
        headers.append(VARY, HeaderValue::from_static("X-Inertia"));
    }
}

#[derive(Debug)]
struct ResponseError(String);

impl ResponseError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ResponseError {}

#[cfg(test)]
mod tests {
    use http::header::{ACCEPT_ENCODING, VARY};

    use super::*;

    #[test]
    fn vary_preserves_existing_values_and_deduplicates_inertia() {
        let mut headers = HeaderMap::new();
        headers.append(VARY, HeaderValue::from_static("Accept-Encoding"));
        headers.append(VARY, HeaderValue::from_static("x-inertia, Origin"));

        add_vary(&mut headers);

        let values = headers
            .get_all(VARY)
            .iter()
            .map(|value| value.to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(values, ["Accept-Encoding", "x-inertia, Origin"]);
        assert!(!headers.contains_key(ACCEPT_ENCODING));
    }
}
