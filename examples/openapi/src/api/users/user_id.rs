use topcoat::{
    Result,
    context::Cx,
    openapi::{Operation, Parameter},
    router::{StatusCode, content::Json, error::RouterErrorExt, module_param, route},
};

use super::User;

module_param!(user_id: u64 else bad_request);

// The path parameter is documented as a string unless an operation gives it
// a type.
#[route(
    GET,
    with(Operation::new()
        .summary("Look up a user")
        .parameter(
            Parameter::path("user_id")
                .schema::<u64>()
                .description("The ID of the user."),
        )
        .response_with::<Json<User>>(StatusCode::OK, "The user.")
        .response(StatusCode::NOT_FOUND, "No user has this ID."))
)]
async fn show(cx: &Cx) -> Result<Json<User>> {
    let name = match *user_id(cx)? {
        1 => Some("Ada"),
        2 => Some("Grace"),
        _ => None,
    };
    let name = name.ok_or_not_found()?;
    Ok(Json(User {
        id: *user_id(cx)?,
        name: name.to_owned(),
    }))
}
