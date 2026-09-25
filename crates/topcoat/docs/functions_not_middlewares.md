# Functions, not middlewares

Use functions that take `cx: &Cx` to read request data and check application rules. A function can fetch the current user, require authentication, or build on another helper.

Call each helper where its result is needed. Use `#[memoize]` to share expensive results across calls in the same request.

# What not to do

## Middleware

Middleware often pushes authentication away from the code that needs the authenticated user. The middleware authenticates the request and stores the user somewhere the page can find it later, while the page assumes the middleware has already run.

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

The page depends on middleware configured elsewhere. Missing or incorrectly ordered middleware can cause a panic or leave a route without its intended authentication check.

## Extractors

Extractors avoid that hidden setup by putting the auth requirement in the handler signature:

```rust
# struct Auth(User);
# struct User;
# struct Html;
# fn render_account(_: User) -> Html { Html }
async fn account_page(Auth(user): Auth) -> Html {
    render_account(user)
}
```

This is more robust than middleware because the auth requirement is visible. The tradeoff is that every component below the page now needs to receive the user explicitly:

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

Passing data explicitly works well when it belongs to the caller. For request-wide data, intermediate components may have to accept a value only to pass it on.

# What to do in Topcoat

Write composable request functions instead. Each function adds one small piece of logic, accepts `&Cx`, and can be called from any page, layout, or component.

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

`#[memoize]` stores the owned `Option<User>` for the request, but exposes it to callers as `Option<&User>`. That lets downstream helpers borrow the current user without cloning it or threading ownership through the component tree.

Now the component that needs authentication declares it by calling `require_auth(cx)`:

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

`user_avatar` checks authentication whenever it renders. Without a valid session, it returns an unauthorized error. Each component or handler that needs protection calls the helper itself.

The database lookup runs once for the same user ID during a request. Callers can independently request the user and share the cached result.

## Shape the functions by meaning

Use several focused helpers instead of one large auth function:

- `session_cookie(cx)` reads the HTTP headers.
- `fetch_user(cx, user_id)` performs the database lookup and memoizes it.
- `fetch_current_user(cx)` turns the session into optional user data.
- `require_auth(cx)` returns the user or an unauthorized error.

That keeps each function reusable. Public UI can call `fetch_current_user(cx)` and render a signed-out state. Private UI can call `require_auth(cx).await?` and fail closed. Admin UI can build on the same pattern:

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

Use the same pattern for other application data that depends on the request. Use a router layer when work must wrap request handling, such as compressing a response.
