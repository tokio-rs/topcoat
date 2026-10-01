use quote::quote;
use topcoat_core_grammar::testing::assert_deterministic;
use topcoat_router_grammar::{
    layer::Layer, layout::Layout, module_param::ModuleParam, not_found::NotFound, page::Page,
    path_param::PathParam, query_params::QueryParams, route::Route, segment::Segment,
};

#[test]
fn page() {
    let attrs = [
        quote! {},
        quote! { "./export" },
        quote! { "/posts/{post_id}" },
        quote! { [GET, POST] "/either" },
        quote! { * "/anything" },
    ];
    for attr in attrs {
        assert_deterministic(|| {
            Page::parse(
                attr.clone(),
                quote! {
                    async fn show(cx: &Cx, Form(input): Form<Input>) -> Result<impl View> {
                        Ok(view! { <p>(input.name)</p> })
                    }
                },
            )
        });
    }
}

#[test]
fn route() {
    let attrs = [
        quote! { POST },
        quote! { GET "./health" },
        quote! { [GET, POST] "/multi" },
    ];
    for attr in attrs {
        assert_deterministic(|| {
            Route::parse(
                attr.clone(),
                quote! {
                    async fn submit(cx: &Cx, Json(body): Json<Body>) -> Result<String> {
                        Ok(body.text)
                    }
                },
            )
        });
    }
}

#[test]
fn layout() {
    for attr in [quote! {}, quote! { "./admin" }] {
        assert_deterministic(|| {
            Layout::parse(
                attr.clone(),
                quote! {
                    async fn shell(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
                        Ok(view! { <main>(slot)</main> })
                    }
                },
            )
        });
    }
}

#[test]
fn layer() {
    for attr in [quote! {}, quote! { "./v1" }] {
        assert_deterministic(|| {
            Layer::parse(
                attr.clone(),
                quote! {
                    async fn wrap(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
                        next.run(cx, body).await
                    }
                },
            )
        });
    }
}

#[test]
fn not_found() {
    assert_deterministic(|| syn::parse_str::<NotFound>(""));
    assert_deterministic(|| syn::parse_str::<NotFound>(r#""/docs""#));
}

#[test]
fn segment() {
    assert_deterministic(|| syn::parse_str::<Segment>(r#"rename = "documentation""#));
    assert_deterministic(|| syn::parse_str::<Segment>("kind = Group"));
}

#[test]
fn path_param() {
    let inputs = [
        "slug",
        "pub post_id: u64, error = bad_request",
        "*rest",
        "*numbers: u32",
    ];
    for input in inputs {
        assert_deterministic(|| syn::parse_str::<PathParam>(input));
    }
}

#[test]
fn module_param() {
    assert_deterministic(|| syn::parse_str::<ModuleParam>("post_id: u32, error = not_found"));
}

#[test]
fn query_params() {
    for attr in [quote! {}, quote! { error = redirect("?") }] {
        assert_deterministic(|| {
            QueryParams::parse(
                attr.clone(),
                quote! {
                    struct Query {
                        page: Option<u32>,
                        tags: Vec<String>,
                    }
                },
            )
        });
    }
}
