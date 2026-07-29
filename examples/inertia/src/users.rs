use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;
use topcoat::{
    Error, Result,
    context::{Cx, app_context},
    inertia::{Inertia, InertiaResponse, ScrollMetadata, defer, flash, flash_errors, once, scroll},
    router::{
        content::Json,
        error::{SeeOther, see_other},
        route, uri,
    },
};

const PAGE_SIZE: usize = 8;
const SEEDED_USER_COUNT: u64 = 40;

#[derive(Clone, Serialize)]
struct User {
    id: u64,
    name: String,
}

pub struct Users {
    entries: Mutex<Vec<User>>,
    stats_resolutions: AtomicU64,
    navigation_resolutions: AtomicU64,
}

impl Users {
    fn page(&self, requested_page: usize) -> UserPage {
        let entries = self.lock();
        let page_count = entries.len().div_ceil(PAGE_SIZE).max(1);
        let page = requested_page.clamp(1, page_count);
        let start = (page - 1) * PAGE_SIZE;
        let users = entries
            .iter()
            .skip(start)
            .take(PAGE_SIZE)
            .cloned()
            .collect();
        UserPage {
            users,
            page,
            page_count,
        }
    }

    fn create(&self, name: &str) {
        let mut entries = self.lock();
        let id = entries.iter().map(|user| user.id).max().unwrap_or(0) + 1;
        entries.insert(
            0,
            User {
                id,
                name: name.to_owned(),
            },
        );
    }

    fn stats(&self) -> Stats {
        Stats {
            total: self.lock().len(),
            resolution: self.stats_resolutions.fetch_add(1, Ordering::Relaxed) + 1,
        }
    }

    fn navigation(&self) -> Navigation {
        Navigation {
            items: ["Home", "Users", "Create user"],
            resolution: self.navigation_resolutions.fetch_add(1, Ordering::Relaxed) + 1,
        }
    }

    fn lock(&self) -> MutexGuard<'_, Vec<User>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for Users {
    fn default() -> Self {
        Self {
            entries: Mutex::new(
                (1..=SEEDED_USER_COUNT)
                    .map(|id| User {
                        id,
                        name: format!("User {id}"),
                    })
                    .collect(),
            ),
            stats_resolutions: AtomicU64::new(0),
            navigation_resolutions: AtomicU64::new(0),
        }
    }
}

struct UserPage {
    users: Vec<User>,
    page: usize,
    page_count: usize,
}

#[derive(Serialize)]
struct Stats {
    total: usize,
    resolution: u64,
}

#[derive(Serialize)]
struct Navigation {
    items: [&'static str; 3],
    resolution: u64,
}

#[derive(Deserialize)]
struct NewUser {
    name: String,
}

#[route(GET "/users")]
async fn index(cx: &Cx) -> Result<InertiaResponse> {
    let page = state(cx).page(page_number(cx));
    let navigation = once(async { Ok::<_, Error>(state(cx).navigation()) })
        .as_key("main-navigation")
        .until(Duration::from_mins(10));

    Inertia::new("Users/Index")
        .prop_with(
            "users",
            scroll(page.users, scroll_metadata(page.page, page.page_count)),
        )
        .prop_with(
            "stats",
            defer(async { Ok::<_, Error>(state(cx).stats()) }).rescue(),
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

    state(cx).create(name);
    flash(cx, "notice", format!("Created {name}"))?;
    Ok(see_other("/users"))
}

fn state(cx: &Cx) -> &Users {
    app_context(cx)
}

fn page_number(cx: &Cx) -> usize {
    uri(cx)
        .query()
        .and_then(|query| query.split('&').find_map(|pair| pair.strip_prefix("page=")))
        .and_then(|page| page.parse().ok())
        .unwrap_or(1)
}

fn scroll_metadata(page: usize, page_count: usize) -> ScrollMetadata {
    let page = u64::try_from(page).expect("page numbers fit into u64");
    let page_count = u64::try_from(page_count).expect("page counts fit into u64");
    ScrollMetadata::new("page")
        .current_page(page)
        .previous_page((page > 1).then_some(page - 1))
        .next_page((page < page_count).then_some(page + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_users_are_persisted_and_counted() {
        let users = Users::default();

        users.create("Ada");

        let first_page = users.page(1);
        assert_eq!(first_page.users[0].name, "Ada");
        assert_eq!(first_page.users.len(), PAGE_SIZE);
        assert_eq!(first_page.page_count, 6);
        assert_eq!(users.stats().total, 41);
    }
}
