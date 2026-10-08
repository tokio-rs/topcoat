use topcoat_router::RouterBuilder;

use crate::SupportedLocales;

/// Installs localization on a [`RouterBuilder`].
///
/// Import this trait to call [`supported_locales`](Self::supported_locales).
pub trait RouterBuilderL10nExt {
    /// Registers the locales the application serves on the app context.
    ///
    /// Passing an array of locales makes the first one the fallback. Use
    /// [`SupportedLocales::with_fallback`] to choose another.
    ///
    /// # Panics
    ///
    /// Panics when the supported locales are registered twice.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use topcoat::{
    ///     l10n::{RouterBuilderL10nExt, SupportedLocales, locale},
    ///     router::Router,
    /// };
    ///
    /// pub fn router() -> Router {
    ///     Router::builder()
    ///         .supported_locales(
    ///             SupportedLocales::new([locale!("de"), locale!("fr")]).with_fallback(locale!("en")),
    ///         )
    ///         .build()
    /// }
    /// ```
    #[must_use]
    fn supported_locales(self, locales: impl Into<SupportedLocales>) -> Self;
}

impl RouterBuilderL10nExt for RouterBuilder {
    #[track_caller]
    fn supported_locales(self, locales: impl Into<SupportedLocales>) -> Self {
        self.app_context(locales.into())
    }
}

#[cfg(test)]
mod tests {
    use topcoat_core::context::{Cx, app_context};
    use topcoat_router::{
        Body, Method, Methods, Path, Route, RouteFuture, RouteId, Router, request::Request,
        response::Response,
    };

    use super::*;
    use crate::locale;

    struct Fallback;

    impl Route for Fallback {
        fn id(&self) -> RouteId {
            static ID: std::sync::LazyLock<RouteId> = std::sync::LazyLock::new(RouteId::new);
            *ID
        }

        fn methods(&self) -> Methods<'_> {
            Methods::Only(&[Method::GET])
        }

        fn path(&self) -> &Path {
            Path::ROOT
        }

        fn handle<'cx>(&'cx self, cx: &'cx Cx, _body: Body) -> RouteFuture<'cx> {
            Box::pin(async move {
                let fallback = app_context::<SupportedLocales>(cx).fallback().to_string();
                Ok(Response::new(Body::from(fallback)))
            })
        }
    }

    #[tokio::test]
    async fn handlers_read_the_registered_locales() {
        let router = Router::builder()
            .route(Fallback)
            .supported_locales([locale!("de"), locale!("en")])
            .build();
        let request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("request should build");

        let response = router.handle(request).await;

        let body = topcoat_router::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        assert_eq!(body, "de");
    }
}
