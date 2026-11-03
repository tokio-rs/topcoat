use topcoat::{
    Result,
    context::Cx,
    router::{content::Json, route},
};

use crate::{SignedJson, User};

#[route(POST)]
pub(crate) async fn signed(_cx: &Cx, SignedJson(user): SignedJson<User>) -> Result<Json<User>> {
    Ok(Json(user))
}
