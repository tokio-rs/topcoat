use http::header::{CONTENT_LENGTH, CONTENT_TYPE, COOKIE, LOCATION, SET_COOKIE, VARY};
use http::{HeaderValue, Method, Request, StatusCode};
use serde_json::{Value, json};
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::cookie::{Key, RouterBuilderCookieExt};
use topcoat::inertia::{
    CookieFlashStore, Inertia, InertiaConfig, Page, RouterBuilderInertiaExt,
    clear_history_on_redirect, flash, flash_errors, header, inertia_location, inertia_root,
    preserve_fragment_on_redirect, share,
};
use topcoat::router::error::see_other;
use topcoat::router::{Body, IntoResponse, Response, RouteFn, RouteFuture, Router, to_bytes};
use topcoat::view::{HtmlContext, NodeViewParts, PartsWriter, View, ViewParts};

fn root(cx: &Cx, page: &Page) -> View {
    let mut parts = ViewParts::new();
    let mut writer = PartsWriter::new(&mut parts, HtmlContext::Text);
    writer.push_str_unescaped("<!DOCTYPE html><html><body>");
    inertia_root(page).into_view_parts(cx, &mut writer);
    writer.push_str_unescaped("</body></html>");
    View::new(parts)
}

async fn load_auth(cx: &Cx) -> Result<Value> {
    std::future::ready(()).await;
    Ok(json!({"request": format!("{:?}", cx.id())}))
}

fn page_handler(cx: &Cx, _body: Body) -> RouteFuture<'_> {
    Box::pin(async move {
        Inertia::new("Home")
            .prop(
                "unsafe",
                "</script><tag> & \"quotes\" and Gruesse aus Zuerich",
            )
            .render(cx)
            .await?
            .into_response(cx)
    })
}

fn location_handler(cx: &Cx, _body: Body) -> RouteFuture<'_> {
    Box::pin(async move { inertia_location("/login").into_response(cx) })
}

fn api_handler(cx: &Cx, _body: Body) -> RouteFuture<'_> {
    Box::pin(async move { "ok".into_response(cx) })
}

fn empty_handler(cx: &Cx, _body: Body) -> RouteFuture<'_> {
    Box::pin(async move { ().into_response(cx) })
}

fn redirect_handler(cx: &Cx, _body: Body) -> RouteFuture<'_> {
    Box::pin(async move { see_other("/target#details").into_response(cx) })
}

fn external_redirect_handler(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
    Box::pin(async move {
        let mut response = Response::new(Body::from("stale body"));
        *response.status_mut() = StatusCode::FOUND;
        response.headers_mut().insert(
            LOCATION,
            HeaderValue::from_static("https://other.test/away"),
        );
        response
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
        response
            .headers_mut()
            .insert(CONTENT_LENGTH, HeaderValue::from_static("10"));
        response
            .headers_mut()
            .append(SET_COOKIE, HeaderValue::from_static("session=kept; Path=/"));
        Ok(response)
    })
}

fn temporary_external_redirect_handler(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
    Box::pin(async move {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::TEMPORARY_REDIRECT;
        response.headers_mut().insert(
            LOCATION,
            HeaderValue::from_static("https://other.test/temporary"),
        );
        Ok(response)
    })
}

fn found_handler(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
    Box::pin(async move {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::FOUND;
        response
            .headers_mut()
            .insert(LOCATION, HeaderValue::from_static("/target"));
        Ok(response)
    })
}

fn precedence_handler(cx: &Cx, _body: Body) -> RouteFuture<'_> {
    Box::pin(async move {
        share(cx, "source", "request")?;
        Inertia::new("Precedence")
            .prop("source", "page")
            .render(cx)
            .await?
            .into_response(cx)
    })
}

fn reserved_errors_handler(cx: &Cx, _body: Body) -> RouteFuture<'_> {
    Box::pin(async move {
        Inertia::new("Invalid")
            .prop("errors", json!({"field": "invalid"}))
            .render(cx)
            .await?
            .into_response(cx)
    })
}

fn mutation_handler(cx: &Cx, _body: Body) -> RouteFuture<'_> {
    Box::pin(async move {
        flash_errors(cx, json!({"email": "Already taken"}))?;
        flash(cx, "notice", "Please correct the form")?;
        clear_history_on_redirect(cx)?;
        preserve_fragment_on_redirect(cx)?;
        see_other("/").into_response(cx)
    })
}

fn protocol_router(key: Key) -> Router {
    let config = InertiaConfig::new(root)
        .root_id("inertia-app")
        .nonce_with(|_| Some("request-nonce".to_owned()))
        .share_with(|cx, props| {
            props.lazy("auth", load_auth(cx));
            Ok(())
        })
        .flash_store(CookieFlashStore::new().secure(false));

    Router::builder()
        .route(RouteFn::new(Method::GET, "/", page_handler))
        .route(RouteFn::new(Method::GET, "/location", location_handler))
        .route(RouteFn::new(Method::GET, "/api", api_handler))
        .route(RouteFn::new(Method::GET, "/empty", empty_handler))
        .route(RouteFn::new(Method::GET, "/fragment", redirect_handler))
        .route(RouteFn::new(
            Method::GET,
            "/external",
            external_redirect_handler,
        ))
        .route(RouteFn::new(Method::POST, "/users", mutation_handler))
        .route(RouteFn::new(Method::GET, "/precedence", precedence_handler))
        .app_context(key)
        .inertia(config)
        .cookies()
        .build()
}

async fn body(response: Response) -> String {
    String::from_utf8(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}

async fn json_body(response: Response) -> Value {
    serde_json::from_str(&body(response).await).unwrap()
}

fn inertia_request(path: &str) -> http::request::Builder {
    Request::builder()
        .uri(path)
        .header(&header::X_INERTIA, "true")
}

fn cookie_pair(response: &Response) -> String {
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .next()
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

#[tokio::test]
async fn ordinary_response_uses_the_v3_script_bootstrap() {
    let router = protocol_router(Key::generate());
    let response = router
        .handle(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(VARY)
            .and_then(|value| value.to_str().ok()),
        Some("X-Inertia")
    );
    let html = body(response).await;
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains(
        "<script data-page=\"inertia-app\" type=\"application/json\" nonce=\"request-nonce\">"
    ));
    assert!(html.contains("\\u003c/script>\\u003ctag>"));
    assert!(html.contains("Gruesse aus Zuerich"));
    assert!(html.contains("<div id=\"inertia-app\"></div>"));
    assert!(!html.contains("<div id=\"inertia-app\" data-page="));
}

#[tokio::test]
async fn inertia_response_is_json_with_shared_errors() {
    let router = protocol_router(Key::generate());
    let response = router
        .handle(inertia_request("/?page=2").body(Body::empty()).unwrap())
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(&header::X_INERTIA)
            .and_then(|value| value.to_str().ok()),
        Some("true")
    );
    assert_eq!(
        response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/json; charset=utf-8")
    );
    let page = json_body(response).await;
    assert_eq!(page["component"], "Home");
    assert_eq!(page["url"], "/?page=2");
    assert_eq!(page["version"], Value::Null);
    assert_eq!(page["props"]["errors"], json!({}));
    assert!(page["props"]["auth"].is_object());
    assert_eq!(page["sharedProps"], json!(["errors", "auth"]));
    assert!(page.get("flash").is_none());
    assert!(page.get("encryptHistory").is_none());
}

#[tokio::test]
async fn location_response_branches_by_request_type() {
    let router = protocol_router(Key::generate());
    let ordinary = router
        .handle(
            Request::builder()
                .uri("/location")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(ordinary.status(), StatusCode::FOUND);
    assert_eq!(ordinary.headers().get(LOCATION).unwrap(), "/login");

    let inertia = router
        .handle(inertia_request("/location").body(Body::empty()).unwrap())
        .await;
    assert_eq!(inertia.status(), StatusCode::CONFLICT);
    assert_eq!(
        inertia.headers().get(&header::X_INERTIA_LOCATION).unwrap(),
        "/login"
    );
    assert!(!inertia.headers().contains_key(&header::X_INERTIA));
}

#[tokio::test]
async fn unrelated_responses_do_not_gain_inertia_vary() {
    let router = protocol_router(Key::generate());
    let response = router
        .handle(Request::builder().uri("/api").body(Body::empty()).unwrap())
        .await;

    assert!(!response.headers().contains_key(VARY));
    assert_eq!(body(response).await, "ok");
}

#[tokio::test]
async fn empty_response_redirects_back_only_to_a_safe_referer() {
    let router = Router::builder()
        .route(RouteFn::new(Method::GET, "/empty", empty_handler))
        .base_url("https://example.test")
        .app_context(Key::generate())
        .inertia(InertiaConfig::new(root).flash_store(CookieFlashStore::new().secure(false)))
        .cookies()
        .build();
    let same_origin = router
        .handle(
            inertia_request("/empty")
                .header("Referer", "https://example.test/form")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(same_origin.status(), StatusCode::CONFLICT);
    assert_eq!(
        same_origin
            .headers()
            .get(&header::X_INERTIA_LOCATION)
            .unwrap(),
        "https://example.test/form"
    );

    let cross_origin = router
        .handle(
            inertia_request("/empty?retry=1")
                .header("Referer", "https://other.test/form")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(
        cross_origin
            .headers()
            .get(&header::X_INERTIA_LOCATION)
            .unwrap(),
        "/empty?retry=1"
    );
}

#[tokio::test]
async fn stale_version_and_redirect_extensions_use_v3_headers() {
    let router = Router::builder()
        .route(RouteFn::new(Method::GET, "/", page_handler))
        .route(RouteFn::new(Method::GET, "/fragment", redirect_handler))
        .route(RouteFn::new(
            Method::GET,
            "/external",
            external_redirect_handler,
        ))
        .base_url("https://example.test")
        .app_context(Key::generate())
        .inertia(
            InertiaConfig::new(root)
                .version("server-v2")
                .convert_external_redirects(true)
                .flash_store(CookieFlashStore::new().secure(false)),
        )
        .cookies()
        .build();

    let stale = router
        .handle(
            inertia_request("/?page=2")
                .header(&header::X_INERTIA_VERSION, "client-v1")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    assert_eq!(
        stale.headers().get(&header::X_INERTIA_LOCATION).unwrap(),
        "https://example.test/?page=2"
    );
    assert_eq!(
        stale.headers().get(&header::X_INERTIA_VERSION).unwrap(),
        "server-v2"
    );

    let fragment = router
        .handle(
            inertia_request("/fragment")
                .header(&header::X_INERTIA_VERSION, "server-v2")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(fragment.status(), StatusCode::CONFLICT);
    assert_eq!(
        fragment.headers().get(&header::X_INERTIA_REDIRECT).unwrap(),
        "/target#details"
    );
    assert!(!fragment.headers().contains_key(LOCATION));

    let external = router
        .handle(
            inertia_request("/external")
                .header(&header::X_INERTIA_VERSION, "server-v2")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(external.status(), StatusCode::CONFLICT);
    assert_eq!(
        external.headers().get(&header::X_INERTIA_LOCATION).unwrap(),
        "https://other.test/away"
    );
    assert!(!external.headers().contains_key(CONTENT_TYPE));
    assert!(!external.headers().contains_key(CONTENT_LENGTH));
    assert_eq!(
        external.headers().get(SET_COOKIE).unwrap(),
        "session=kept; Path=/"
    );
    assert!(body(external).await.is_empty());
}

#[tokio::test]
async fn mutation_redirects_are_normalized_to_see_other() {
    let router = Router::builder()
        .route(RouteFn::new(Method::PUT, "/put", found_handler))
        .route(RouteFn::new(Method::PATCH, "/patch", found_handler))
        .route(RouteFn::new(Method::DELETE, "/delete", found_handler))
        .app_context(Key::generate())
        .inertia(InertiaConfig::new(root).flash_store(CookieFlashStore::new().secure(false)))
        .cookies()
        .build();

    for (method, path) in [
        (Method::PUT, "/put"),
        (Method::PATCH, "/patch"),
        (Method::DELETE, "/delete"),
    ] {
        let response = router
            .handle(
                inertia_request(path)
                    .method(method)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers().get(LOCATION).unwrap(), "/target");
    }
}

#[tokio::test]
async fn prefetch_and_temporary_redirects_keep_normal_http_semantics() {
    let router = Router::builder()
        .route(RouteFn::new(Method::GET, "/fragment", redirect_handler))
        .route(RouteFn::new(
            Method::GET,
            "/temporary",
            temporary_external_redirect_handler,
        ))
        .base_url("https://example.test")
        .app_context(Key::generate())
        .inertia(
            InertiaConfig::new(root)
                .convert_external_redirects(true)
                .flash_store(CookieFlashStore::new().secure(false)),
        )
        .cookies()
        .build();

    let prefetch = router
        .handle(
            inertia_request("/fragment")
                .header(&header::PURPOSE, "prefetch")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(prefetch.status(), StatusCode::SEE_OTHER);
    assert_eq!(prefetch.headers().get(LOCATION).unwrap(), "/target#details");
    assert!(!prefetch.headers().contains_key(&header::X_INERTIA_REDIRECT));

    let temporary = router
        .handle(inertia_request("/temporary").body(Body::empty()).unwrap())
        .await;
    assert_eq!(temporary.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        temporary.headers().get(LOCATION).unwrap(),
        "https://other.test/temporary"
    );
    assert!(
        !temporary
            .headers()
            .contains_key(&header::X_INERTIA_LOCATION)
    );
}

#[tokio::test]
async fn explicit_location_wins_over_a_stale_asset_version() {
    let router = Router::builder()
        .route(RouteFn::new(Method::GET, "/location", location_handler))
        .app_context(Key::generate())
        .inertia(
            InertiaConfig::new(root)
                .version("server-v2")
                .flash_store(CookieFlashStore::new().secure(false)),
        )
        .cookies()
        .build();
    let response = router
        .handle(
            inertia_request("/location")
                .header(&header::X_INERTIA_VERSION, "client-v1")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        response.headers().get(&header::X_INERTIA_LOCATION).unwrap(),
        "/login"
    );
    assert!(!response.headers().contains_key(&header::X_INERTIA_VERSION));
}

#[tokio::test]
async fn page_props_override_all_shared_prop_sources() {
    let config = InertiaConfig::new(root)
        .share_with(|_, props| {
            props.prop("source", "configured");
            Ok(())
        })
        .flash_store(CookieFlashStore::new().secure(false));
    let router = Router::builder()
        .route(RouteFn::new(Method::GET, "/", precedence_handler))
        .app_context(Key::generate())
        .inertia(config)
        .cookies()
        .build();
    let response = router
        .handle(inertia_request("/").body(Body::empty()).unwrap())
        .await;
    let page = json_body(response).await;

    assert_eq!(page["props"]["source"], "page");
    assert_eq!(page["sharedProps"], json!(["errors", "source"]));
}

#[tokio::test]
async fn errors_is_reserved_for_the_validation_channel() {
    let router = Router::builder()
        .route(RouteFn::new(Method::GET, "/", reserved_errors_handler))
        .app_context(Key::generate())
        .inertia(InertiaConfig::new(root).flash_store(CookieFlashStore::new().secure(false)))
        .cookies()
        .build();
    let response = router
        .handle(inertia_request("/").body(Body::empty()).unwrap())
        .await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let router = Router::builder()
        .route(RouteFn::new(Method::GET, "/", page_handler))
        .app_context(Key::generate())
        .inertia(
            InertiaConfig::new(root)
                .share_with(|_, props| {
                    props.prop("errors", json!({"field": "invalid"}));
                    Ok(())
                })
                .flash_store(CookieFlashStore::new().secure(false)),
        )
        .cookies()
        .build();
    let response = router
        .handle(inertia_request("/").body(Body::empty()).unwrap())
        .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn private_flash_crosses_redirects_and_is_consumed_once() {
    let key = Key::generate();
    let first_app = protocol_router(key.clone());
    let redirect = first_app
        .handle(
            inertia_request("/users")
                .method(Method::POST)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(redirect.status(), StatusCode::SEE_OTHER);
    let cookie = cookie_pair(&redirect);
    assert!(!cookie.contains("Already taken"));
    assert!(!cookie.contains("Please correct"));

    let second_app = protocol_router(key);
    let page = second_app
        .handle(
            inertia_request("/")
                .header(COOKIE, &cookie)
                .header(&header::X_INERTIA_ERROR_BAG, "createUser")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(page.status(), StatusCode::OK);
    let removal_cookie = cookie_pair(&page);
    let page = json_body(page).await;
    assert_eq!(
        page["props"]["errors"]["createUser"]["email"],
        "Already taken"
    );
    assert_eq!(page["flash"]["notice"], "Please correct the form");
    assert_eq!(page["clearHistory"], true);
    assert_eq!(page["preserveFragment"], true);

    let consumed = second_app
        .handle(
            inertia_request("/")
                .header(COOKIE, removal_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    let consumed = json_body(consumed).await;
    assert_eq!(consumed["props"]["errors"], json!({}));
    assert!(consumed.get("flash").is_none());
}
