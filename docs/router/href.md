Builds the URL of a page or route. Name the handler instead of writing its URL by hand, so links, form actions, and redirects keep working when you move a module or change a path.

# Linking to a handler

Pass the handler as the first argument and put the result in a URL attribute such as `href` or `action`.

```rust
use topcoat::{Result, context::Cx, router::{href, page}, view::{View, view}};

#[page]
async fn home(cx: &Cx) -> Result<impl View> {
    Ok(view! { <a href=(href!(about::page))>"About"</a> })
}

mod about {
    use super::*;

    #[page]
    pub async fn page(cx: &Cx) -> Result<impl View> {
        Ok(view! { <h1>"About"</h1> })
    }
}
# fn main() {}
```

With [module routing](macro.module_router.html), this links to `/about`. Handlers with an explicit path work the same way. A handler can also link to itself by its own name.

For other sites, or for a link to a fragment on the same page such as `href="#details"`, write the URL as a plain string.

# Path parameters

When the handler's path has parameters, add one `name: value` argument for each of them after the handler. The name is the one declared with [`path_param!`](macro.path_param.html) or [`module_param!`](macro.module_param.html). The order of the arguments does not matter.

```rust
// src/app/posts.rs
use topcoat::{
    Result,
    context::Cx,
    router::{error::bad_request, href, module_param, page},
    view::{View, view},
};

#[page]
async fn posts(cx: &Cx) -> Result<impl View> {
    // Links to /posts/42.
    Ok(view! { <a href=(href!(post_id::post, post_id: 42))>"A post"</a> })
}

// src/app/posts/post_id.rs
pub mod post_id {
    use super::*;

    module_param!(pub post_id: u64 else bad_request);

    #[page]
    pub async fn post(cx: &Cx) -> Result<impl View> {
        let post_id = post_id(cx)?;
        Ok(view! { <a href=(href!(post, post_id: post_id))>"Permalink"</a> })
    }
}
# fn main() {}
```

Inside the module that declares a parameter, use its plain name. From the parent of a `module_param!` module, the plain name works too, as in `posts` above. Anywhere else, name the parameter by its path, such as `crate::app::posts::post_id`.

Each value is written with [`Display`](https://doc.rust-lang.org/core/fmt/trait.Display.html) and percent-encoded, so `slug: "a/b"` stays inside its own segment as `a%2Fb`. See [`path_param!`](macro.path_param.html) for the values each kind of parameter accepts, including lists for catch-all parameters.

Building the URL panics if a parameter of the path gets no value, or if an argument names a parameter the path does not have.

# Queries and fragments

Call [`query`](struct.Href.html#method.query) with any value that implements serde's `Serialize` to add it to the query string. You can call it several times to combine values. Call [`fragment`](struct.Href.html#method.fragment) to set the part after `#`.

```rust
use serde::Serialize;
use topcoat::{Result, context::Cx, router::{href, page}, view::{View, view}};

#[derive(Serialize)]
struct Pagination {
    page: u32,
}

#[page]
async fn posts(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        // Links to /posts?page=2.
        <a href=(href!(posts).query(Pagination { page: 2 }))>"Next page"</a>
        // Links to /posts#archive.
        <a href=(href!(posts).fragment("archive"))>"Archive"</a>
    })
}
# fn main() {}
```

# Absolute URLs

URLs are relative to the site root by default, like `/posts/42`. Call [`absolute`](struct.Href.html#method.absolute) to include the scheme and host, like `https://example.com/posts/42`. Some parts of an app, such as mail rendering, make absolute URLs the default. Call [`relative`](struct.Href.html#method.relative) there if you want the short form.

# Getting the URL as a string

Views render an [`Href`](struct.Href.html) directly. Everywhere else, call [`resolve`](struct.Href.html#method.resolve) with the context to get a `String`, for example to redirect after a form submission.

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{error::{SeeOther, see_other}, href, route},
};
# #[topcoat::router::page("/posts")]
# async fn posts(cx: &Cx) -> Result<impl topcoat::view::View> { Ok(topcoat::view::view! { "Posts" }) }

#[route(POST "/posts/new")]
async fn create_post(cx: &Cx) -> Result<SeeOther> {
    Ok(see_other(href!(posts).resolve(cx)))
}
```

# Marking the current page

Call [`is_current`](struct.Href.html#method.is_current) to check whether a link points at the page being served. This is useful for highlighting the active item in a navigation bar.

```rust
use topcoat::{
    Result,
    context::Cx,
    router::href,
    view::{View, class, component, view},
};
# #[topcoat::router::page("/posts")]
# async fn posts(cx: &Cx) -> Result<impl View> { Ok(topcoat::view::view! { "Posts" }) }

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

A link without a query stays current when the page is shown with any query, so it stays active while the list is filtered or paginated. A link with a query is only current when the request has the same query. The fragment is ignored.

# Other targets

The first argument can also be a [`Path`](struct.Path.html) or a path string such as `"/posts/{post_id}"`. `href!` reads a plain name as a handler, so wrap a `Path` constant in parentheses, as in `href!((POSTS))`.

Each handler gives `href!` a different type, so links to different handlers cannot go into the same list. Store the handlers as `&dyn HrefTarget` instead and call `href!` on each one. Add `Sync` so the view can move between threads.

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{HrefTarget, href},
    view::{View, component, view},
};
# #[topcoat::router::page("/")]
# async fn home(cx: &Cx) -> Result<impl View> { Ok(topcoat::view::view! { "Home" }) }
# #[topcoat::router::page("/posts")]
# async fn posts(cx: &Cx) -> Result<impl View> { Ok(topcoat::view::view! { "Posts" }) }

const NAV: &[(&str, &(dyn HrefTarget + Sync))] = &[("Home", &home), ("Posts", &posts)];

#[component]
async fn nav(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <nav>
            for (label, target) in NAV {
                let link = href!(*target);
                let current = link.is_current(cx);
                <a href=(link) aria-current=(current.then_some("page"))>(*label)</a>
            }
        </nav>
    })
}
```

This only works when all handlers in the list take the same path parameters. Here neither takes any.
