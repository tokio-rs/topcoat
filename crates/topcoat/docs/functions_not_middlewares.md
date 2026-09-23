# Functions, not middlewares

Put request-specific application logic in functions that take `cx: &Cx`. Call a helper where its result is needed, so that the requirement stays with the code that depends on it.

For example, a component that displays private account data can call `require_auth(cx)`. That helper checks authentication every time the component is used. Use [`#[memoize]`](macro@memoize) to share expensive work within the request.

## Share an authentication result

Keep optional authentication and required authentication in separate helpers. Public pages can handle a missing user, while private content requires one.

The example below assumes your app provides `authenticate`, which validates the request's credentials and loads its user:

```rust
use topcoat::{
    Result,
    context::{Cx, memoize},
    router::error::{RouterErrorExt, UnauthorizedError},
};
#
# struct User { name: String }
# async fn authenticate(_: &Cx) -> Option<User> { None }

#[memoize(as_ref)]
async fn current_user(cx: &Cx) -> Option<User> {
    authenticate(cx).await
}

async fn require_auth(cx: &Cx) -> Result<&User, UnauthorizedError> {
    current_user(cx).await.ok_or_unauthorized()
}
```

`current_user` caches its result for the request. The `as_ref` option lets callers borrow the cached user. Repeated calls share the authentication work.

## Check access where it is needed

Call the helper before reading or rendering private data:

```rust
use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};
#
# use topcoat::router::error::UnauthorizedError;
# struct User { name: String }
# async fn require_auth(_: &Cx) -> Result<&User, UnauthorizedError> {
#     Err(topcoat::router::error::unauthorized())
# }

#[component]
async fn account_name(cx: &Cx) -> Result<impl View> {
    let user = require_auth(cx).await?;
    Ok(view! { <span>(user.name.as_str())</span> })
}
```

If authentication fails, `?` returns the error before the component renders private content. The caller does not need to pass a user through every intermediate component.

Authentication identifies a user. Add authorization checks when access also depends on permissions or ownership:

```rust
use topcoat::{Result, context::Cx, router::error::RouterErrorExt};
#
# struct User;
# impl User { fn is_admin(&self) -> bool { false } }
# async fn require_auth(_: &Cx) -> Result<&User> {
#     Err(topcoat::router::error::unauthorized().into())
# }

async fn require_admin(cx: &Cx) -> Result<&User> {
    let user = require_auth(cx).await?;
    Ok(user.is_admin().then_some(user).ok_or_forbidden()?)
}
```

Use the same pattern for other request data. Give each helper one purpose, and let it call other helpers as needed. Router layers remain useful for behavior that wraps request handling, such as tracing.
