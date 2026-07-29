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

#[derive(Clone, Serialize)]
struct OptimisticUser {
    id: u64,
    name: String,
    age: u8,
}

pub struct Users {
    entries: Mutex<Vec<User>>,
    optimistic_entries: Mutex<Vec<OptimisticUser>>,
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

    fn optimistic_users(&self) -> Vec<OptimisticUser> {
        self.optimistic_entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn create_optimistic_user(&self, name: &str, age: u8) {
        let mut entries = self
            .optimistic_entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let id = entries.iter().map(|user| user.id).max().unwrap_or(0) + 1;
        entries.insert(
            0,
            OptimisticUser {
                id,
                name: name.to_owned(),
                age,
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
            items: ["Home", "Users", "Create user", "Optimistic updates"],
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
            optimistic_entries: Mutex::new(vec![
                OptimisticUser {
                    id: 1,
                    name: "Ada".to_owned(),
                    age: 36,
                },
                OptimisticUser {
                    id: 2,
                    name: "Grace".to_owned(),
                    age: 42,
                },
            ]),
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
    items: [&'static str; 4],
    resolution: u64,
}

#[derive(Deserialize)]
struct NewUser {
    name: String,
}

#[derive(Deserialize)]
struct NewOptimisticUser {
    name: String,
    age: String,
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

#[route(GET "/optimistic")]
async fn optimistic(cx: &Cx) -> Result<InertiaResponse> {
    Inertia::new("Optimistic")
        .prop("optimisticUsers", state(cx).optimistic_users())
        .render(cx)
        .await
}

#[route(POST "/optimistic")]
async fn optimistic_store(cx: &Cx, Json(input): Json<NewOptimisticUser>) -> Result<SeeOther> {
    // Keep the temporary row visible long enough to inspect in the local demo.
    tokio::time::sleep(Duration::from_millis(750)).await;

    let name = input.name.trim();
    let age = input
        .age
        .parse::<u8>()
        .ok()
        .filter(|age| (1..=120).contains(age));
    let mut errors = serde_json::Map::new();
    if name.len() < 2 {
        errors.insert("name".to_owned(), json!("Enter at least two characters"));
    }
    if age.is_none() {
        errors.insert("age".to_owned(), json!("Enter an age from 1 to 120"));
    }
    if !errors.is_empty() {
        flash_errors(cx, errors)?;
        return Ok(see_other("/optimistic"));
    }

    state(cx).create_optimistic_user(name, age.expect("age was validated"));
    Ok(see_other("/optimistic"))
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

    #[test]
    fn optimistic_users_are_stored_separately() {
        let users = Users::default();

        users.create_optimistic_user("Lin", 28);

        let optimistic_users = users.optimistic_users();
        assert_eq!(optimistic_users[0].name, "Lin");
        assert_eq!(optimistic_users[0].age, 28);
        assert_eq!(users.stats().total, 40);
    }
}
