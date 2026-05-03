use axum::response::Redirect;
use topcoat_core::context::{Cx, abort};

pub async fn redirect(cx: &Cx, uri: &str) -> ! {
    abort(cx, Box::new(Redirect::temporary(uri))).await
}

pub async fn redirect_permanent(cx: &Cx, uri: &str) -> ! {
    abort(cx, Box::new(Redirect::permanent(uri))).await
}

pub trait RedirectExt {
    type T;

    fn unwrap_or_redirect(self, cx: &Cx, uri: &str) -> impl Future<Output = Self::T>;
    fn unwrap_or_redirect_permanent(self, cx: &Cx, uri: &str) -> impl Future<Output = Self::T>;
}

impl<T> RedirectExt for Option<T> {
    type T = T;

    async fn unwrap_or_redirect(self, cx: &Cx, uri: &str) -> T {
        match self {
            Some(value) => value,
            None => redirect(cx, uri).await,
        }
    }

    async fn unwrap_or_redirect_permanent(self, cx: &Cx, uri: &str) -> T {
        match self {
            Some(value) => value,
            None => redirect_permanent(cx, uri).await,
        }
    }
}

impl<T, E> RedirectExt for Result<T, E> {
    type T = T;

    async fn unwrap_or_redirect(self, cx: &Cx, uri: &str) -> T {
        match self {
            Ok(value) => value,
            Err(_) => redirect(cx, uri).await,
        }
    }

    async fn unwrap_or_redirect_permanent(self, cx: &Cx, uri: &str) -> T {
        match self {
            Ok(value) => value,
            Err(_) => redirect_permanent(cx, uri).await,
        }
    }
}
