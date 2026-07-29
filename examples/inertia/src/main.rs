mod root;
mod users;

use topcoat::{
    Error, Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    cookie::{Key, RouterBuilderCookieExt},
    inertia::{CookieFlashStore, Inertia, InertiaConfig, InertiaResponse, RouterBuilderInertiaExt},
    router::{Router, RouterBuilderDiscoverExt, route},
    session::{RouterBuilderSessionExt, SessionConfig},
};

#[tokio::main]
async fn main() {
    let assets = AssetBundle::load().unwrap();
    let config = InertiaConfig::new(root::root)
        .root_id("inertia-app")
        .version_from_assets(assets.catalog())
        .share_with(|_cx, props| {
            props.lazy("auth", async {
                Ok::<_, Error>(serde_json::json!({"name": "Ada"}))
            });
            Ok(())
        })
        .flash_store(CookieFlashStore::new().secure(false));

    let secret = std::env::var("TOPCOAT_COOKIE_KEY")
        .unwrap_or_else(|_| "local-inertia-example-key-change-me-before-production".to_owned());
    let router = Router::builder()
        .discover()
        .app_context(Key::derive_from(secret.as_bytes()))
        .app_context(users::Users::default())
        .inertia(config)
        .cookies()
        .sessions(SessionConfig::default())
        .assets(assets)
        .build();

    topcoat::start(router).await.unwrap();
}

#[route(GET "/")]
async fn home(cx: &Cx) -> Result<InertiaResponse> {
    Inertia::new("Home")
        .prop("greeting", "Topcoat with Inertia.js v3")
        .render(cx)
        .await
}
