# Functions, not middlewares

In other frameworks, you often guard routes with middlewares or extractors, for example to reject invalid requests or users who are not signed in. In Topcoat, prefer short, composable functions that take `cx: &Cx` and do the validation or data fetching directly.

You can call these functions from anywhere in the component tree, without coupling unrelated components together. For data fetching and other expensive work, add [`#[memoize]`](macro@memoize) so that repeated calls within a request run the work only once.

# What not to do

## Middleware

Middleware moves authentication away from the code that needs the authenticated user. The middleware authenticates the request and stores the user somewhere the page can find it later. The page then assumes that the middleware has already run.

```rust
# struct Request { extensions: Extensions }
# struct Extensions;
# impl Extensions { fn insert<T>(&mut self, _v: T) {} fn get<T>(&self) -> Option<&T> { None } }
# struct Html;
# struct User;
# async fn authenticate(_: &Request) -> User { User }
# fn render_account(_: &User) -> Html { Html }
async fn auth_middleware(request: &mut Request) {
    let user = authenticate(request).await;
    request.extensions.insert(user);
}

async fn account_page(request: &Request) -> Html {
    let user = request
        .extensions
        .get::<User>()
        .expect("auth middleware must have run");

    render_account(user)
}
```

This works, but the page now depends on configuration that lives somewhere else. If the middleware is missing or runs in the wrong order, the handler panics. If someone adds a protected route and forgets the middleware, the route can expose private data.

## Extractors

Extractors avoid this hidden setup by putting the auth requirement in the handler signature:

```rust
# struct Auth(User);
# struct User;
# struct Html;
# fn render_account(_: User) -> Html { Html }
async fn account_page(Auth(user): Auth) -> Html {
    render_account(user)
}
```

This is more robust than middleware, because the auth requirement is visible. The downside is that every component below the page now has to receive the user as an argument:

```rust
# struct Auth(User);
# #[derive(Clone)] struct User { avatar_url: u8 }
# struct Html;
# fn render(_: Html, _: Html) -> Html { Html }
# async fn account_settings(_: User) -> Html { Html }
# fn render_image(_: u8) -> Html { Html }
async fn account_page(Auth(user): Auth) -> Html {
    render(
        account_sidebar(user.clone()).await,
        account_settings(user.clone()).await,
    )
}

async fn account_sidebar(user: User) -> Html {
    user_avatar(user).await
}

async fn user_avatar(user: User) -> Html {
    render_image(user.avatar_url)
}
```

Passing arguments is fine for local data, but the current user belongs to the whole request. Passing it through every layout and component couples unrelated code, just so that a deeply nested component can ask a simple question.

# What to do in Topcoat

Write small request functions instead. Each function adds one piece of logic, takes `cx: &Cx`, and can be called from any page, layout, or component.

```rust
use topcoat::{
    context::{app_context, memoize, Cx},
    router::{error::{RouterErrorExt, UnauthorizedError}, request::headers},
    Result,
};

# #[derive(Clone)] struct Db;
# struct User;
# struct FetchBuilder;
# impl User { fn fetch_by_id(_: &str) -> FetchBuilder { FetchBuilder } }
# impl FetchBuilder { async fn exec(self, _: Db) -> Option<User> { None } }
#
/// Returns the application database handle.
fn db(cx: &Cx) -> Db {
    app_context::<Db>(cx).clone()
}

/// Fetches a user by ID, deduplicated for the duration of the request.
#[memoize(as_ref)]
async fn fetch_user(cx: &Cx, user_id: &str) -> Option<User> {
    User::fetch_by_id(user_id).exec(db(cx)).await
}

/// Reads the session ID from the request cookies.
fn session_cookie(cx: &Cx) -> Option<&str> {
    let headers = headers(cx);
    // ... extract session cookie from HTTP headers
    None
}

/// Resolves the current user from the session, if one exists.
async fn fetch_current_user(cx: &Cx) -> Option<&User> {
    let user_id = session_cookie(cx)?;
    fetch_user(cx, user_id).await
}

/// Returns the current user or falls through to Topcoat's unauthorized response.
async fn require_auth(cx: &Cx) -> Result<&User, UnauthorizedError> {
    fetch_current_user(cx).await.ok_or_unauthorized()
}
```

With `as_ref`, `#[memoize]` stores the owned `Option<User>` for the request and returns it to callers as `Option<&User>`. Other helpers can then borrow the current user without cloning it.

A component that needs authentication now says so by calling `require_auth(cx)`:

```rust
use topcoat::{
    context::Cx,
    view::{View, component, view},
    Result,
};

# use topcoat::router::error::UnauthorizedError;
# struct User { avatar_url: &'static str, name: &'static str }
# async fn require_auth(_: &Cx) -> Result<&User, UnauthorizedError> { Err(topcoat::router::error::unauthorized()) }
#
/// Renders the current user's avatar and requires authentication wherever it is used.
#[component]
async fn user_avatar(cx: &Cx) -> Result<impl View> {
    let user = require_auth(cx).await?;

    Ok(view! {
        <img
            src=(user.avatar_url)
            alt=(format!("{}'s avatar", user.name))
        >
    })
}
```

`user_avatar` is now guarded wherever it is used. If a page renders it without a valid session, the error propagates and the response becomes Topcoat's unauthorized response. The requirement lives with the code that depends on it, so you do not have to remember to guard every route that might render the component.

Because `fetch_user` is memoized, the database lookup runs at most once per user ID during a request. A layout can call `fetch_current_user(cx)` to render the navigation, a page can call `require_auth(cx)` to protect private content, and a nested component can call `require_auth(cx)` again to render an avatar. The calls stay independent of each other, and the expensive work still runs only once.

## Shape the functions by meaning

Split the logic into several focused helpers instead of one large auth function:

- `session_cookie(cx)` reads the HTTP headers.
- `fetch_user(cx, user_id)` performs the database lookup and memoizes it.
- `fetch_current_user(cx)` turns the session into optional user data.
- `require_auth(cx)` turns a missing user into an unauthorized error.

Each function stays reusable. Public UI can call `fetch_current_user(cx)` and render a signed-out state. Private UI can call `require_auth(cx).await?` and fail when there is no user. Admin UI can build on the same pattern:

```rust
use topcoat::{context::Cx, router::error::RouterErrorExt, Result};

# use topcoat::router::error::UnauthorizedError;
# struct User;
# impl User { fn is_admin(&self) -> bool { false } }
# async fn require_auth(_: &Cx) -> Result<&User, UnauthorizedError> { Err(topcoat::router::error::unauthorized()) }
#
/// Returns the current user if they have admin permissions.
async fn require_admin(cx: &Cx) -> Result<&User> {
    let user = require_auth(cx).await?;
    Ok(user.is_admin().then_some(user).ok_or_forbidden()?)
}
```

The same style works beyond auth. Feature flags, tenant lookup, locale detection, experiments, settings, and data derived from the URL all fit well as `cx` functions. Use a router layer for concerns that apply to every request at the HTTP level, such as compression, tracing, or low-level request normalization. Use `cx` functions when application code needs request-scoped data.
