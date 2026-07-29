use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;
use topcoat::{
    Error, Result,
    context::Cx,
    inertia::{Inertia, InertiaResponse, ScrollMetadata, defer, flash, flash_errors, once, scroll},
    router::{
        content::Json,
        error::{SeeOther, see_other},
        route, uri,
    },
};

#[derive(Serialize)]
struct User {
    id: u64,
    name: String,
}

#[derive(Deserialize)]
struct NewUser {
    name: String,
}

#[route(GET "/users")]
async fn index(cx: &Cx) -> Result<InertiaResponse> {
    let page = page_number(cx);
    let navigation = once(async { Ok::<_, Error>(["Home", "Users", "Create user"]) })
        .as_key("main-navigation")
        .until(Duration::from_mins(10));
    let navigation = if uri(cx).query() == Some("refresh_navigation=1") {
        navigation.fresh()
    } else {
        navigation
    };

    Inertia::new("Users/Index")
        .prop_with("users", scroll(users(page), scroll_metadata(page)))
        .prop_with(
            "stats",
            defer(async { Ok::<_, Error>(json!({"total": 9})) }).rescue(),
        )
        .prop_with(
            "activity",
            defer(async { Ok::<_, Error>(["signed in", "viewed users"]) })
                .group("dashboard")
                .merge(),
        )
        .prop_with("navigation", navigation)
        .render(cx)
        .await
}

#[route(GET "/users/create")]
async fn create(cx: &Cx) -> Result<InertiaResponse> {
    Inertia::new("Users/Create").render(cx).await
}

#[route(POST "/users")]
async fn store(cx: &Cx, Json(input): Json<NewUser>) -> Result<SeeOther> {
    let name = input.name.trim();
    if name.len() < 2 {
        flash_errors(cx, json!({"name": "Enter at least two characters"}))?;
        return Ok(see_other("/users/create"));
    }

    flash(cx, "notice", format!("Created {name}"))?;
    Ok(see_other("/users"))
}

fn page_number(cx: &Cx) -> u64 {
    uri(cx)
        .query()
        .and_then(|query| query.split('&').find_map(|pair| pair.strip_prefix("page=")))
        .and_then(|page| page.parse().ok())
        .unwrap_or(1)
        .clamp(1, 3)
}

fn users(page: u64) -> Vec<User> {
    let first = (page - 1) * 3 + 1;
    (first..first + 3)
        .map(|id| User {
            id,
            name: format!("User {id}"),
        })
        .collect()
}

fn scroll_metadata(page: u64) -> ScrollMetadata {
    ScrollMetadata::new("page")
        .current_page(page)
        .previous_page((page > 1).then_some(page - 1))
        .next_page((page < 3).then_some(page + 1))
}
