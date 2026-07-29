use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use topcoat_cookie::{Cookie, CookieJarCell, Cookies, Key, SameSite, private_cookies, time};
use topcoat_core::context::{Cx, try_app_context, try_request_context};
use topcoat_core::error::Result;

const DEFAULT_NAME: &str = "topcoat-inertia-flash";
const DEFAULT_MAX_AGE: Duration = Duration::from_mins(5);
const COOKIE_SIZE_LIMIT: usize = 3_800;
const ENCRYPTION_AND_ATTRIBUTES_ALLOWANCE: usize = 256;

pub type FlashStoreFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

pub trait FlashStore: Send + Sync {
    fn read<'a>(&'a self, cx: &'a Cx) -> FlashStoreFuture<'a, Option<Vec<u8>>>;

    fn write<'a>(&'a self, cx: &'a Cx, payload: &'a [u8]) -> FlashStoreFuture<'a, ()>;

    fn delete<'a>(&'a self, cx: &'a Cx) -> FlashStoreFuture<'a, ()>;
}

#[derive(Debug, Clone)]
pub struct CookieFlashStore {
    name: String,
    secure: bool,
    max_age: Duration,
}

impl CookieFlashStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: DEFAULT_NAME.to_owned(),
            secure: true,
            max_age: DEFAULT_MAX_AGE,
        }
    }

    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    #[must_use]
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    #[must_use]
    pub fn max_age(mut self, max_age: Duration) -> Self {
        self.max_age = max_age;
        self
    }

    fn ensure_context(cx: &Cx) -> Result<()> {
        if try_request_context::<CookieJarCell>(cx).is_none() {
            return Err(FlashError::new(
                "the Inertia cookie flash store requires `.cookies()` on the router",
            )
            .into());
        }
        if try_app_context::<Key>(cx).is_none() {
            return Err(FlashError::new(
                "the Inertia cookie flash store requires a persistent cookie `Key` as app context",
            )
            .into());
        }
        Ok(())
    }
}

impl Default for CookieFlashStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FlashStore for CookieFlashStore {
    fn read<'a>(&'a self, cx: &'a Cx) -> FlashStoreFuture<'a, Option<Vec<u8>>> {
        Box::pin(async move {
            Self::ensure_context(cx)?;
            let Some(cookie) = private_cookies(cx).get(&self.name) else {
                return Ok(None);
            };
            Ok(URL_SAFE_NO_PAD.decode(cookie.value()).ok())
        })
    }

    fn write<'a>(&'a self, cx: &'a Cx, payload: &'a [u8]) -> FlashStoreFuture<'a, ()> {
        Box::pin(async move {
            Self::ensure_context(cx)?;
            let encoded = URL_SAFE_NO_PAD.encode(payload);
            let projected_size = self
                .name
                .len()
                .saturating_add(encoded.len())
                .saturating_add(ENCRYPTION_AND_ATTRIBUTES_ALLOWANCE);
            if projected_size > COOKIE_SIZE_LIMIT {
                return Err(FlashError::new(
                    "Inertia flash data is too large for the private cookie store",
                )
                .into());
            }
            let max_age = time::Duration::try_from(self.max_age)
                .map_err(|_| FlashError::new("Inertia flash cookie max age is too large"))?;
            let cookie = Cookie::build((self.name.clone(), encoded))
                .http_only(true)
                .same_site(SameSite::Lax)
                .path("/")
                .secure(self.secure)
                .max_age(max_age)
                .build();
            private_cookies(cx).add(cookie);
            Ok(())
        })
    }

    fn delete<'a>(&'a self, cx: &'a Cx) -> FlashStoreFuture<'a, ()> {
        Box::pin(async move {
            Self::ensure_context(cx)?;
            let cookie = Cookie::build(self.name.clone()).path("/").build();
            private_cookies(cx).remove(cookie);
            Ok(())
        })
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub(crate) struct FlashPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) errors: Option<Value>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) clear_history: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) preserve_fragment: bool,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub(crate) data: Map<String, Value>,
}

impl FlashPayload {
    pub(crate) fn is_empty(&self) -> bool {
        self.errors.is_none()
            && !self.clear_history
            && !self.preserve_fragment
            && self.data.is_empty()
    }

    pub(crate) fn merge(&mut self, other: Self) {
        if other.errors.is_some() {
            self.errors = other.errors;
        }
        self.clear_history |= other.clear_history;
        self.preserve_fragment |= other.preserve_fragment;
        self.data.extend(other.data);
    }
}

#[derive(Debug, Default)]
pub(crate) struct FlashState {
    inner: Mutex<FlashStateInner>,
}

#[derive(Debug, Default)]
struct FlashStateInner {
    incoming: FlashPayload,
    pending: FlashPayload,
}

impl FlashState {
    pub(crate) fn new(incoming: FlashPayload) -> Self {
        Self {
            inner: Mutex::new(FlashStateInner {
                incoming,
                pending: FlashPayload::default(),
            }),
        }
    }

    pub(crate) fn incoming(&self) -> FlashPayload {
        self.lock().incoming.clone()
    }

    pub(crate) fn combined(&self) -> FlashPayload {
        let state = self.lock();
        let mut payload = state.incoming.clone();
        payload.merge(state.pending.clone());
        payload
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, FlashStateInner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct IncludedIncomingFlash;

/// Adds validation errors to the redirect-scoped flash payload.
///
/// # Errors
///
/// Returns an error when `errors` cannot be serialized as a JSON object or
/// when the Inertia layer is not installed for the request.
pub fn flash_errors(cx: &Cx, errors: impl Serialize) -> Result<()> {
    let errors = serde_json::to_value(errors)?;
    if !errors.is_object() {
        return Err(
            FlashError::new("Inertia validation errors must serialize as an object").into(),
        );
    }
    state(cx)?.lock().pending.errors = Some(errors);
    Ok(())
}

/// Adds a page-level flash value to the redirect-scoped payload.
///
/// # Errors
///
/// Returns an error when `value` cannot be serialized or when the Inertia
/// layer is not installed for the request.
pub fn flash(cx: &Cx, key: impl Into<String>, value: impl Serialize) -> Result<()> {
    state(cx)?
        .lock()
        .pending
        .data
        .insert(key.into(), serde_json::to_value(value)?);
    Ok(())
}

/// Clears browser history when the redirect target page is rendered.
///
/// # Errors
///
/// Returns an error when the Inertia layer is not installed for the request.
pub fn clear_history_on_redirect(cx: &Cx) -> Result<()> {
    state(cx)?.lock().pending.clear_history = true;
    Ok(())
}

/// Preserves the current fragment when the redirect target page is rendered.
///
/// # Errors
///
/// Returns an error when the Inertia layer is not installed for the request.
pub fn preserve_fragment_on_redirect(cx: &Cx) -> Result<()> {
    state(cx)?.lock().pending.preserve_fragment = true;
    Ok(())
}

fn state(cx: &Cx) -> Result<&FlashState> {
    try_request_context(cx).ok_or_else(|| {
        FlashError::new("Inertia flash helpers require `.inertia(...)` on the router").into()
    })
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Debug)]
struct FlashError(String);

impl FlashError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for FlashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FlashError {}
