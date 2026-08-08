Topcoat generates URLs from the pages that serve them. `href!` builds a page's URL from its path parameters, so a link names the handler it points at instead of a string that goes stale when the route moves.

```rust
use topcoat::{Result, router::{href, page}, view::view};

#[page]
async fn home() -> Result {
    view! {
        <a href=(href!(about))>"About"</a>
    }
}
```

# Building a URL

`href!` takes the page's handler name, then one value per path parameter, in the order the parameters appear in the path.

```rust
href!(about)                                  // "/about"
href!(post, PostId(42))                       // "/posts/42"
href!(user, OrganizationId(9), UserId(41))    // "/organizations/9/users/41"
```

Path parameters are the [`path_param!`](../crates/topcoat-router/macro/docs/path_param.md) newtypes the page already reads from the request, so one type serves both directions.

```rust
// src/app/posts/post_id.rs
path_param!(pub post_id: u64, error = not_found);

#[page]
async fn post(cx: &Cx) -> Result {
    view! { <h1>"Post " (path_param::<PostId>(cx)?)</h1> }
}
```

`path_param!` declares the parameter by its URL name and generates the type, `PostId` here, with the visibility the declaration gives it. It replaces today's `#[path_param]` attribute, which cannot declare the generic that a string parameter needs to be constructible. This document assumes that change; its design lands next.

A page that links to `post` imports it alongside the type.

```rust
// src/app/posts.rs
use crate::app::posts::post_id::{PostId, post};

#[page]
async fn index() -> Result {
    view! {
        <ul>
            for id in [1_u64, 2, 3] {
                <li><a href=(href!(post, PostId(id)))>"Post " (id)</a></li>
            }
        </ul>
    }
}
```

# What href! returns

An `Href`, not a `String`. Resolving one needs the route table and the base URL, both of which live on the request context, so an `Href` describes a URL and builds it later. That is also why it cannot implement `Display`.

A view resolves it for you. `Href` implements `AttributeValueViewParts` and `NodeViewParts`, emitting each part of the URL as its own `ViewPart` the way `Class<T>` does, so rendering writes the segments into the response without building a string first.

```rust
<a href=(href!(post, PostId(42)))>"Post"</a>
```

A page marker renders as its own URL, which covers the parameterless links that make up most of a site.

```rust
<a href=(about)>"About"</a>
```

Elsewhere an `Href` takes a context.

```rust
href!(post, PostId(42)).resolve(cx)      // "/posts/42"
```

`query`, `fragment`, and `absolute` each return an `Href`, so they chain onto one before it resolves.

# Href::new

`href!` expands to `Href::new`, passing the page marker that `#[page]` already generates and the parameters as a tuple.

```rust
href!(user, OrganizationId(9), UserId(41))
Href::new(user, (OrganizationId(9), UserId(41)))
```

The constructor takes a single argument for the parameters because Rust has no variadic functions, so any number of them means a tuple. Zero and one read worst, and zero is what most pages take.

```rust
Href::new(about, ())
Href::new(post, (PostId(42),))
```

`href!` spreads its arguments into that tuple, keeping it out of every link. A rendered page marker goes through `Href::new(about, ())` as well, so a page that later gains a path parameter fails there the way any link missing a parameter does.

`Href::new` stays the real API. It keeps URL building on `Href` instead of a method generated onto every page, and it lets a helper build a URL for a page it takes as an argument. The rest of this document applies to both forms.

# Parameter values

A parameter declared without a type is unparsed: its segment is a string the page reads as is. Reading it needs no more than an unsized `str`, but a link has to construct one, so the generated type is generic over anything that borrows as a string.

```rust
path_param!(pub slug);      // pub struct Slug<T: AsRef<str>>(pub T);

href!(show, Slug("my-first-post"))    // &str
href!(show, Slug(post.slug))          // String
```

`path_param::<Slug>(cx)` still borrows the decoded segment out of the request, so serving a request allocates nothing. The allocation moves to the link, where the value usually came out of a `String` already.

Each value is percent-encoded into its segment, so a title or an address containing `/`, `?`, or `#` stays inside the segment it belongs to.

```rust
href!(show, Slug("hello/world"))     // "/posts/hello%2Fworld"
```

A catch-all segment (`{*path}`) stands for several segments, so it is the one case where `/` is preserved.

```rust
href!(document, DocPath("guides/getting-started"))    // "/docs/guides/getting-started"
```

An empty value leaves nothing between the slashes, and the `/posts/` it produces no longer matches the route it was built from, so it is rejected when the URL is built.

Group segments never appear in a served URL and take no value, so a page under `app::_marketing::pricing` is reached with `href!(pricing)`.

Linking to a page from outside its own module needs its parameter type there, so `path_param!` takes a visibility, which the generated field carries. A parameter linked only from its own subtree can stay private.

# When mistakes are caught

Parameters are checked when the URL is built, never at compile time. A macro sees only its own item, so a page cannot name parameters its ancestors declare. Explicit paths could be checked earlier, but one rule for every page beats a partial one.

A URL carries the source location it was built at, so the failure names the link rather than the page rendering it.

```text
page `app::posts::post_id::post` serves "/posts/{post_id}" and needs `post_id`, but was given `user_id`
  link built at src/app/posts.rs:31:22
```

An unregistered page reads the same way. It usually means a page with an explicit path in an application that never called `.discover()`, or a module not reachable through a `mod` declaration from the module router's root.

```text
no route registered for page `app::posts::post_id::post`
  link built at src/app/posts.rs:31:22
```

These are programming errors with no recovery, so they panic rather than render a broken link, the same way a missing [asset](../crates/topcoat/docs/asset.md) does. A view renders to a complete string before its response is built, so the panic never truncates a partly written response.

# Query strings and fragments

`query` appends a query string from any `serde::Serialize` value. `#[query_params]` structs derive `Serialize`, so the struct a page reads is the struct a link writes.

```rust
#[query_params]
pub struct PostsQuery {
    page: Option<u32>,
    q: Option<String>,
}

// "/posts?page=2&q=rust"
href!(index).query(&PostsQuery {
    page: Some(2),
    q: Some("rust".into()),
})
```

Fields holding `None` are left out. A slice of pairs covers one-off links that do not deserve a type.

```rust
href!(index).query(&[("page", 2)])
```

`fragment` appends a `#` fragment.

```rust
href!(post, PostId(42)).fragment("comments")      // "/posts/42#comments"
```

# Relative and absolute URLs

Whether an `Href` renders relative or absolute is the rendering context's call. A page renders root-relative, which is what a link inside the site needs. A context that knows its output leaves the site renders absolute, so every `Href` in a [mail](../crates/topcoat/docs/mail.md) body comes out absolute without the link asking for it. `absolute` and `relative` force a mode where the context cannot know: a feed or a sitemap renders as a page, but its URLs are read elsewhere.

```rust
href!(post, PostId(42)).absolute()      // "https://example.com/posts/42"
```

The scheme and host come from the [base URL](../crates/topcoat/docs/context.md) registered on the router.

```rust
let router = Router::builder().base_url("https://example.com").build();
```

A router without one falls back to the address it serves on, so links resolve in development without configuration.

An application mounted under a path prefix registers it as part of that base URL, as in `https://example.com/app`. The proxy strips the prefix before the router matches, so the router only ever sees `/posts/42` while the browser needs `/app/posts/42`. Relative URLs carry the prefix for that reason, so every `Href` reads the base URL.

# Redirecting

A Post/Redirect/Get handler names its destination the way a link does.

```rust
use topcoat::router::error::{SeeOther, see_other};

#[route(POST "/posts")]
async fn create(Form(input): Form<NewPost>) -> Result<SeeOther> {
    let id = insert(input).await?;
    Ok(see_other(href!(post, PostId(id))))
}
```

`see_other` takes an `Href` and resolves it while building the response, where the context is available.

# Resolving outside a view

`resolve` needs a context, and a handler already holds one.

```rust
#[route(GET "/feed.xml")]
async fn feed(cx: &Cx) -> Result<String> {
    Ok(href!(post, PostId(42)).absolute().resolve(cx))
}
```

Work that runs outside a request, such as a background job or a sitemap task, takes a context from the router itself.

```rust
let cx = router.cx();
let url = href!(post, PostId(42)).absolute().resolve(&cx);
```

A test takes one from the application's router. A module-derived URL resolves through the route table, so a context built by hand covers only pages with an explicit path.

```rust
let cx = app::router().cx();
assert_eq!(href!(post, PostId(42)).resolve(&cx), "/posts/42");
```

# API routes

`#[route]` handlers work the same way, which covers form actions and fetch targets.

```rust
#[route(POST "/api/posts/{post_id}/publish")]
async fn publish(cx: &Cx) -> Result<&'static str> {
    Ok("published")
}

href!(publish, PostId(42))        // "/api/posts/42/publish"
```

A `#[layout]` has no URL of its own, so it cannot be linked to.

# Other ideas

These are worth considering but not settled.

**Take the query string and fragment as macro arguments.** A named argument would keep a whole URL in one call rather than splitting it between a macro and a method.

```rust
href!(index, query: PostsQuery { page: 2, q: "rust" })
href!(index, query: { "page": 2 })
href!(post, PostId(42), fragment: "comments")
```

`query: PostsQuery { .. }` names the type but is not a struct literal, since a link sets the fields it cares about and leaves the rest out. It would expand the way Toasty's `create!` does, against a builder generated by `#[query_params]`. The methods stay either way, so this is sugar over them.

**Assertions for links in tests.** An assertion that takes an `Href` could resolve it against the router itself, and compare part by part rather than as one string, so a failure names the segment or query parameter that differs.

**Check parameters at compile time.** Giving `module_router!` the route tree changes what the macro knows. It can pull in each module body itself, thread a module's parameters down to its children, and generate a typed constructor per page.

```rust
module_router! {
    mod about;
    mod posts { mod post_id; }
}
```

Every page then takes its parameters directly.

```rust
Href::new(post, PostId(42))       // one argument per path parameter
Href::new(post, UserId(42))       // does not compile
```

That drops the tuple and most of the reason `href!` exists, and it is the only approach that catches a bad link without running the code that builds it. The cost is that route modules compile as one unit and editor tooling follows them less well, so it is worth revisiting once the dynamic form ships.
