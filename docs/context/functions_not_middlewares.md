# Functions, not middlewares

Put request checks in functions that take `cx: &Cx`. A helper can read request data, load the current user, or check whether that user is allowed to continue.

Call the helper from the code that needs the check or its result. Add `#[memoize]` when several callers should share expensive work within a request.

# What not to do

## Middleware

With authentication middleware, the check and the code using the user often live in different places. The middleware stores a user on the request. The page then reads that value and assumes the check has already happened:

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

For this page to work, the right middleware must run first. Missing it or setting the wrong order can cause a panic or leave a route without its authentication check.

## Extractors

An extractor puts the authentication requirement in the handler's parameters:

```rust
# struct Auth(User);
# struct User;
# struct Html;
# fn render_account(_: User) -> Html { Html }
async fn account_page(Auth(user): Auth) -> Html {
    render_account(user)
}
```

The handler now states that it needs an authenticated user. Components below it still need the user passed to them:

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

Passing values this way is useful for data chosen by the caller. For data shared across a request, it can mean passing a value through components that never use it themselves.

# What to do in Topcoat

Write small helpers that take `&Cx` and call one another. Any page, layout, or component can use them:

```rust
use topcoat::{
    context::{context, memoize, Cx},
    router::{error::{RouterErrorExt, UnauthorizedError}, request::headers},
    Result,
};

# #[derive(Clone)] struct Db;
# struct User;
# struct FetchBuilder;
# impl User { fn fetch_by_id(_: &str) -> FetchBuilder { FetchBuilder } }
# impl FetchBuilder { async fn exec(self, _: Db) -> Option<User> { None } }
#
/// Clones the database handle stored in the context.
fn db(cx: &Cx) -> Db {
    context::<Db>(cx).clone()
}

/// Loads a user by ID and caches the result for this request.
#[memoize(as_ref)]
async fn fetch_user(cx: &Cx, user_id: &str) -> Option<User> {
    User::fetch_by_id(user_id).exec(db(cx)).await
}

/// Finds the session cookie in the request.
fn session_cookie(cx: &Cx) -> Option<&str> {
    let headers = headers(cx);
    // ... extract session cookie from HTTP headers
    None
}

/// Looks up the user named by the session, if present.
async fn fetch_current_user(cx: &Cx) -> Option<&User> {
    let user_id = session_cookie(cx)?;
    fetch_user(cx, user_id).await
}

/// Returns the user, or an unauthorized error if there is none.
async fn require_auth(cx: &Cx) -> Result<&User, UnauthorizedError> {
    fetch_current_user(cx).await.ok_or_unauthorized()
}
```

`#[memoize(as_ref)]` stores the `Option<User>` for the request and gives callers an `Option<&User>`. Helpers can borrow the user without cloning it or passing ownership from one component to the next.

A component that needs an authenticated user calls `require_auth(cx)` itself:

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
/// Checks authentication and renders the user's avatar.
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

Every render of `user_avatar` checks authentication. If the request has no valid session, the component returns an unauthorized error. Other components and handlers that need protection must also call the helper.

Calls with the same user ID and context dependencies share the database lookup. Each caller can ask for the user without knowing whether another caller has already loaded it.

## Shape the functions by meaning

Give each helper one job:

- `session_cookie(cx)` finds the session cookie in the headers.
- `fetch_user(cx, user_id)` loads and caches the user.
- `fetch_current_user(cx)` finds the user for the session, if there is one.
- `require_auth(cx)` requires that user to be present.

Choose the helper that fits the caller. A public page can use `fetch_current_user(cx)` and show a signed-out view when there is no user. A private page can use `require_auth(cx).await?` to stop with an error. An admin check can build on that helper:

```rust
use topcoat::{context::Cx, router::error::RouterErrorExt, Result};

# use topcoat::router::error::UnauthorizedError;
# struct User;
# impl User { fn is_admin(&self) -> bool { false } }
# async fn require_auth(_: &Cx) -> Result<&User, UnauthorizedError> { Err(topcoat::router::error::unauthorized()) }
#
/// Requires the current user to be an admin.
async fn require_admin(cx: &Cx) -> Result<&User> {
    let user = require_auth(cx).await?;
    Ok(user.is_admin().then_some(user).ok_or_forbidden()?)
}
```

This approach also works for other data and rules tied to a request. Use a router layer for work that needs to wrap request handling, such as response compression.
