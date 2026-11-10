pub(crate) mod user_id;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::Cx,
    openapi::Operation,
    router::{StatusCode, content::Json, query_params, route},
};

/// A person with an account.
#[derive(Serialize, JsonSchema)]
pub(crate) struct User {
    pub(crate) id: u64,
    pub(crate) name: String,
}

#[derive(Deserialize, JsonSchema)]
struct CreateUser {
    /// The name to show for the user.
    name: String,
}

// Deriving `JsonSchema` lets `Operation::query` document each field as a
// query parameter. The doc comments become the parameter descriptions.
#[query_params(error = bad_request)]
#[derive(JsonSchema)]
struct ListQuery {
    /// The page to show, starting at 1.
    page: Option<u32>,
}

#[route(
    GET,
    with(Operation::new()
        .summary("List users")
        .query::<ListQuery>()
        .response_with::<Json<Vec<User>>>(StatusCode::OK, "The users on the page."))
)]
async fn list(cx: &Cx) -> Result<Json<Vec<User>>> {
    let query = query_params::<ListQuery>(cx)?;
    let users = match query.page.unwrap_or(1) {
        1 => vec![
            User {
                id: 1,
                name: "Ada".to_owned(),
            },
            User {
                id: 2,
                name: "Grace".to_owned(),
            },
        ],
        _ => Vec::new(),
    };
    Ok(Json(users))
}

#[route(
    POST,
    with(Operation::new()
        .summary("Create a user")
        .request::<Json<CreateUser>>()
        .response_with::<Json<User>>(StatusCode::CREATED, "The new user."))
)]
async fn create(cx: &Cx, Json(input): Json<CreateUser>) -> Result<(StatusCode, Json<User>)> {
    let user = User {
        id: 3,
        name: input.name,
    };
    Ok((StatusCode::CREATED, Json(user)))
}
