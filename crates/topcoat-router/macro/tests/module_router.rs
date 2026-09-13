//! `module_router!` derives each handler's path from its module. The test
//! binary is the root module, so a handler in `settings` is served at
//! `/settings`, and one declared there with `./export` at `/settings/export`.

use topcoat::{
    Result,
    router::{Router, Slot, layout, page},
    view::{View, view},
};

mod common;
use common::{send, send_full};

fn router() -> Router {
    topcoat::router::module_router!().build()
}

// -- module-derived paths --

#[layout]
async fn shell(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! { <main>(slot)</main> })
}

#[page]
async fn home() -> Result<impl View> {
    Ok(view! { "home" })
}

mod blog_posts {
    use topcoat::{
        Result,
        router::page,
        view::{View, view},
    };

    #[page]
    async fn index() -> Result<impl View> {
        Ok(view! { "blog posts" })
    }
}

mod _marketing {
    mod pricing {
        use topcoat::{
            Result,
            router::page,
            view::{View, view},
        };

        #[page]
        async fn pricing() -> Result<impl View> {
            Ok(view! { "pricing" })
        }
    }
}

mod posts {
    mod post_id {
        use topcoat::{
            Result,
            context::Cx,
            router::{page, path_param},
            view::{View, view},
        };

        path_param!(post_id: u32, error = not_found);

        #[page]
        async fn post(cx: &Cx) -> Result<impl View> {
            let id = path_param::<PostId>(cx)?;
            Ok(view! {
                "post "
                (id)
            })
        }
    }
}

mod docs {
    use topcoat::{
        Result,
        router::{page, segment},
        view::{View, view},
    };

    segment!(rename = "documentation");

    #[page]
    async fn docs() -> Result<impl View> {
        Ok(view! { "docs" })
    }
}

#[tokio::test]
async fn root_module_serves_the_root_path() {
    let (status, body) = send(&router(), "/").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<main>home</main>");
}

#[tokio::test]
async fn module_names_are_kebab_cased() {
    let (status, body) = send(&router(), "/blog-posts").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<main>blog posts</main>");
}

#[tokio::test]
async fn group_modules_are_left_out_of_the_url() {
    let router = router();
    let (status, body) = send(&router, "/pricing").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<main>pricing</main>");

    let (status, _) = send(&router, "/_marketing/pricing").await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn path_param_modules_capture_their_segment() {
    let router = router();
    let (status, body) = send(&router, "/posts/42").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<main>post 42</main>");

    let (status, _) = send(&router, "/posts/not-a-number").await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn segment_rename_replaces_the_module_name() {
    let router = router();
    let (status, body) = send(&router, "/documentation").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<main>docs</main>");

    let (status, _) = send(&router, "/docs").await;
    assert_eq!(status, 404);
}

// -- relative paths --

#[page("./about")]
async fn about() -> Result<impl View> {
    Ok(view! { "about" })
}

mod settings {
    use topcoat::{
        Result,
        router::{Slot, layout, page},
        view::{View, view},
    };

    #[page]
    async fn settings() -> Result<impl View> {
        Ok(view! { "settings" })
    }

    #[page("./export")]
    async fn export() -> Result<impl View> {
        Ok(view! { "export" })
    }

    // A layout below the module path wraps only the pages under it, so it
    // coexists with a module-derived layout in the same module.
    #[layout]
    async fn settings_layout(slot: Slot<'_>) -> Result<impl View> {
        Ok(view! { <div>(slot)</div> })
    }

    #[layout("./admin")]
    async fn admin_layout(slot: Slot<'_>) -> Result<impl View> {
        Ok(view! { <section>(slot)</section> })
    }

    mod admin {
        use topcoat::{
            Result,
            router::page,
            view::{View, view},
        };

        #[page]
        async fn admin() -> Result<impl View> {
            Ok(view! { "admin" })
        }
    }
}

mod api {
    use topcoat::{
        Result,
        context::Cx,
        router::{Body, Next, layer, response::Response, route},
    };

    // A module-derived layer covers everything under `/api`.
    #[layer]
    async fn api_header(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
        let mut response = next.run(cx, body).await?;
        response.headers_mut().insert("x-api", "1".parse().unwrap());
        Ok(response)
    }

    // A layer below the module path only runs for the handlers under it.
    #[layer("./v1")]
    async fn version_header(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
        let mut response = next.run(cx, body).await?;
        response
            .headers_mut()
            .insert("x-api-version", "1".parse().unwrap());
        Ok(response)
    }

    #[route(GET "./status")]
    async fn status() -> Result<&'static str> {
        Ok("status")
    }

    mod v1 {
        use topcoat::{Result, router::route};

        #[route(GET "./health")]
        async fn health() -> Result<&'static str> {
            Ok("health")
        }
    }
}

#[tokio::test]
async fn relative_page_at_the_root_module() {
    let (status, body) = send(&router(), "/about").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<main>about</main>");
}

#[tokio::test]
async fn relative_page_sits_next_to_the_module_page() {
    let router = router();
    let (status, body) = send(&router, "/settings").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<main><div>settings</div></main>");

    let (status, body) = send(&router, "/settings/export").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<main><div>export</div></main>");
}

#[tokio::test]
async fn relative_layout_wraps_only_the_pages_below_it() {
    let (status, body) = send(&router(), "/settings/admin").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<main><div><section>admin</section></div></main>");
}

#[tokio::test]
async fn relative_route_is_served_below_the_module() {
    let router = router();
    let (status, body) = send(&router, "/api/status").await;
    assert_eq!(status, 200);
    assert_eq!(body, "status");

    let (status, body) = send(&router, "/api/v1/health").await;
    assert_eq!(status, 200);
    assert_eq!(body, "health");
}

#[tokio::test]
async fn relative_layer_runs_only_below_its_path() {
    let router = router();
    let (_, headers, _) = send_full(&router, "/api/status").await;
    assert_eq!(headers.get("x-api").unwrap(), "1");
    assert!(headers.get("x-api-version").is_none());

    let (_, headers, _) = send_full(&router, "/api/v1/health").await;
    assert_eq!(headers.get("x-api").unwrap(), "1");
    assert_eq!(headers.get("x-api-version").unwrap(), "1");
}
