use serde::de::DeserializeOwned;
use topcoat::{
    Result,
    context::{Cx, context},
    router::{
        Body, ModuleRouterBuilder, Page, Router, RouterBuilder, RouterBuilderDiscoverExt,
        content::Json, href, page, to_bytes,
    },
    view::{View, view},
};

mod common;
use common::send;

trait SiteState: Send + Sync + 'static {
    const NAME: &'static str;
    fn title(&self) -> &str;
}

// No Default impl: constructing the page must not construct its context value.
struct ProductionState(&'static str);
impl SiteState for ProductionState {
    const NAME: &'static str = "production";

    fn title(&self) -> &str {
        self.0
    }
}

struct TestState;
impl SiteState for TestState {
    const NAME: &'static str = "test";

    fn title(&self) -> &'static str {
        "test title"
    }
}

#[page("/")]
async fn home<T: SiteState>(cx: &Cx) -> Result<impl View> {
    let state = context::<T>(cx);
    Ok(view! { <a href=(href!(home::<T>::default()))>(state.title())</a> })
}

fn site_router<T: SiteState>(state: T) -> Router {
    Router::builder()
        .with(state)
        .page(home::<T>::default())
        .build()
}

#[tokio::test]
async fn the_composition_layer_selects_the_context_type() {
    let production = site_router(ProductionState("production title"));
    let test = site_router(TestState);
    assert_eq!(
        send(&production, "/").await,
        (200, "<a href=\"/\">production title</a>".into())
    );
    assert_eq!(
        send(&test, "/").await,
        (200, "<a href=\"/\">test title</a>".into())
    );
}

#[page("/constant")]
async fn constant<const N: usize>() -> Result<impl View> {
    Ok(view! { <p>(N)</p> })
}

#[tokio::test]
async fn const_generic_pages_render() {
    let router = Router::builder().page(constant::<7>).build();
    assert_eq!(send(&router, "/constant").await, (200, "<p>7</p>".into()));
}

struct Title(&'static str);

#[page("/metadata", with(Title(T::NAME)))]
async fn metadata<T, const N: usize>(cx: &Cx) -> Result<impl View>
where
    T: SiteState,
{
    Ok(view! {
        <p>
            (context::<Title>(cx).0)
            (N)
        </p>
    })
}

#[tokio::test]
async fn where_clauses_and_generic_route_context_work() {
    let router = Router::builder()
        .page(metadata::<TestState, 3>::default())
        .build();
    assert_eq!(
        send(&router, "/metadata").await,
        (200, "<p>test3</p>".into())
    );
}

#[page(POST "/body")]
async fn body_page<T>(Json(value): Json<T>) -> Result<impl View>
where
    T: DeserializeOwned + std::fmt::Display + Send + Sync + 'static,
{
    Ok(view! { <p>(value.to_string())</p> })
}

#[tokio::test]
async fn a_generic_page_extracts_its_concrete_body_type() {
    let router = Router::builder()
        .page(body_page::<String>::default())
        .build();
    let request = http::Request::builder()
        .method("POST")
        .uri("/body")
        .header("content-type", "application/json")
        .body(Body::from("\"hello\""))
        .unwrap();
    let response = router.handle(request).await;
    assert_eq!(response.status().as_u16(), 200);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&bytes[..], b"<p>hello</p>");
}

#[page("/composed")]
async fn composed() -> Result<impl View> {
    Ok(view! { <section>home::<ProductionState>()</section> })
}

#[tokio::test]
async fn generic_pages_can_still_be_used_as_components() {
    let router = Router::builder()
        .with(ProductionState("inline"))
        .page(home::<ProductionState>::default())
        .page(composed)
        .build();
    assert_eq!(
        send(&router, "/composed").await,
        (200, "<section><a href=\"/\">inline</a></section>".into())
    );
}

#[test]
fn generic_page_identity_is_stable_and_distinct_per_specialization() {
    let production = home::<ProductionState>::default().id();
    assert_eq!(production, home::<ProductionState>::default().id());
    assert_ne!(production, home::<TestState>::default().id());
    assert_eq!(constant::<1>.id(), constant::<1>.id());
    assert_ne!(constant::<1>.id(), constant::<2>.id());
    assert_ne!(production, constant::<1>.id());
}

#[page("/wrong-link")]
async fn wrong_link() -> Result<impl View> {
    Ok(view! { <a href=(href!(home::<TestState>::default()))>"wrong specialization"</a> })
}

#[tokio::test]
async fn links_cannot_resolve_an_unregistered_specialization() {
    let router = Router::builder()
        .with(ProductionState("production"))
        .page(home::<ProductionState>::default())
        .page(wrong_link)
        .build();
    assert_eq!(send(&router, "/wrong-link").await.0, 500);
}

#[page("/plain")]
async fn plain() -> Result<impl View> {
    Ok(view! { "plain" })
}

#[tokio::test]
async fn discovery_only_collects_non_generic_pages() {
    let router = Router::builder().discover().build();
    assert_eq!(send(&router, "/plain").await, (200, "plain".into()));
    assert_eq!(send(&router, "/").await.0, 404);
    assert_eq!(send(&router, "/constant").await.0, 404);
}

mod module_pages {
    use super::*;

    #[page]
    pub async fn index<T: SiteState>(cx: &Cx) -> Result<impl View> {
        Ok(view! {
            <a href=(href!(details::<T>::default()))>(context::<T>(cx).title())</a>
        })
    }

    #[page("./details")]
    pub async fn details<T: SiteState>() -> Result<impl View> {
        Ok(view! { <p>(T::NAME)</p> })
    }
}

#[tokio::test]
async fn generic_module_pages_can_be_registered_explicitly() {
    let modules = ModuleRouterBuilder::new(module_path!())
        .page(module_pages::index::<TestState>::default())
        .page(module_pages::details::<TestState>::default());
    let router = RouterBuilder::from(modules).with(TestState).build();
    assert_eq!(
        send(&router, "/module-pages").await,
        (
            200,
            "<a href=\"/module-pages/details\">test title</a>".into()
        )
    );
    assert_eq!(
        send(&router, "/module-pages/details").await,
        (200, "<p>test</p>".into())
    );
}

#[tokio::test]
async fn module_discovery_skips_generic_pages() {
    let router = topcoat::router::module_router!().build();
    assert_eq!(send(&router, "/module-pages").await.0, 404);
}
