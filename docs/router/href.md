Builds a URL for a page or route. Use `href!(handler, ...)` for links, form actions, and redirects to Topcoat handlers so their URLs follow the registered paths.

Pass a handler as the first argument to use its registered path, including when linking to the handler from its own body. A [`Path`] expression or path string also works. A bare Rust path is interpreted as a handler name, so pass `Path` constants through the [`href`] function instead.

# Links and form actions

Import the macro from `topcoat::router`. Pass the returned [`Href`] directly to `href`, `action`, or another URL attribute:

```rust
use topcoat::{Result, router::{href, page}, view::{View, view}};

#[page]
async fn home() -> Result<impl View> {
    Ok(view! { <a href=(href!(about::page))>"About"</a> })
}

mod about {
    use super::*;

    #[page]
    pub async fn page() -> Result<impl View> {
        Ok(view! { <h1>"About"</h1> })
    }
}
# fn main() {}
```

With module routing, this links to `/about`. The same syntax works for handlers with explicit paths. Use ordinary URLs for external destinations and standalone fragment links such as `href="#details"`.

# Path parameters

Every further argument fills in one of the path's parameters, with one `path_param!` value per parameter in the order the path declares them, so a link to a post reads as `href!(post, PostId(post.id))`. Each value is written with [`Display`] and percent-encoded, so a parameter declared with a type, like `path_param!(post_id: u64)`, needs that type to implement [`Display`].

Use the returned [`Href`] to configure the query, fragment, or URL form. Render it in a view, or call [`resolve`](Href::resolve) to obtain a string. Use [`is_current`](Href::is_current) to mark a navigation link as active.

```rust
use serde::Serialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, bad_request, see_other},
        href, module_param, page, path_param, route,
    },
    view::{View, view},
};

// src/app/posts.rs
#[derive(Serialize)]
struct Pagination {
    page: u32,
}

#[page]
async fn posts(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <a href=(href!(post_id::post, post_id::PostId(1)))>"The first post"</a>
        <a href=(href!(post_id::post, post_id::PostId(1)).fragment("comments"))>"Its comments"</a>
        <a href=(href!(posts).query(Pagination { page: 2 }))>"Next page"</a>
    })
}

pub mod post_id {
    use super::*;

    module_param!(pub post_id: u64, error = bad_request);

    #[page]
    pub async fn post(cx: &Cx) -> Result<impl View> {
        let post_id = path_param::<PostId>(cx)?;

        Ok(view! {
            <form method="post" action=(href!(publish::publish, PostId(*post_id)))>
                <button>"Publish"</button>
            </form>
        })
    }

    pub mod publish {
        use super::*;

        #[route(POST)]
        pub async fn publish(cx: &Cx) -> Result<SeeOther> {
            let post_id = path_param::<PostId>(cx)?;

            Ok(see_other(href!(post, PostId(*post_id)).resolve(cx)))
        }
    }
}
# fn main() {}
```

Use the [`href`] function to pass parameters as a tuple.

# Active navigation

Use [`Href::is_current`] to check whether a link targets the current path. Use [`class!`](https://docs.rs/topcoat/latest/topcoat/view/macro.class.html) to add its active class alongside its base class:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::href,
    view::{View, class, component, view},
};
# #[topcoat::router::page("/posts")]
# async fn posts() -> Result<impl View> { Ok(topcoat::view::view! { "Posts" }) }

#[component]
async fn posts_link(cx: &Cx) -> Result<impl View> {
    let link = href!(posts);
    let current = link.is_current(cx);

    Ok(view! {
        <a
            href=(link)
            aria-current=(current.then_some("page"))
            class=(class!("nav-link", "active" if current))
        >
            "Posts"
        </a>
    })
}
```
