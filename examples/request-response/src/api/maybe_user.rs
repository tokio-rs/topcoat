use crate::User;
use topcoat::{Result, router::{content::Json, route}};

// Option<Json<T>> is None when the request carries no JSON body, and still
// errors when a malformed body is present.
#[route(POST)]
pub(crate) async fn maybe_user(user: Option<Json<User>>) -> Result<String> {
    match user {
        Some(Json(user)) => Ok(format!("got user {}", user.name)),
        None => Ok("no user provided".to_string()),
    }
}
