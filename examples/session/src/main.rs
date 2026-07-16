use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
};

use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{Router, RouterBuilderDiscoverExt, SeeOther, Slot, layout, page, route, see_other},
    session::{self, RouterBuilderCookieExt, TokenHash},
    view::view,
};

#[tokio::main]
async fn main() {
    topcoat::start(
        Router::builder()
            .sessions(session::Config::default())
            .app_context(Database::default())
            .discover()
            .build(),
    )
    .await
    .unwrap();
}

#[layout("/")]
async fn root(slot: Slot<'_>) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Sessions"</title>
                topcoat::dev::script()
            </head>
            <body>(slot.await?)</body>
        </html>
    }
}

#[page("/")]
async fn page(cx: &Cx) -> Result {
    view! {
        if let Some(user) = current_user(cx) {
            <div>
                "currently logged in as: "
                (&user.name)
            </div>
            <form method="POST" target="/logout"><button>"log out"</button></form>
        } else {
            <div>"currently not logged in"</div>
            <form method="POST" target="/login">
                <input name="name" placeholder="Username" required="true">
                <button>"log in"</button>
            </form>
        }
    }
}

// -----------------------
// API routes

#[derive(Deserialize)]
struct LoginForm {
    name: String,
}

#[route(POST "/login")]
async fn login(cx: &Cx) -> Result<SeeOther> {
    let token_hash = session::start(cx);
    Ok(see_other("/"))
}

#[route(POST "/logout")]
async fn logout(cx: &Cx) -> Result<SeeOther> {
    session::stop(cx);
    Ok(see_other("/"))
}

// -----------------------
// In-memory demo database

#[derive(Debug, Clone)]
struct User {
    name: String,
}

async fn db(cx: &Cx) -> &Database {
    app_context(cx)
}

fn current_user(cx: &Cx) -> Option<User> {
    db(cx).get()
}

#[derive(Debug, Default)]
struct Database {
    users: Mutex<HashMap<TokenHash, User>>,
}

impl Database {
    fn create(&self, token_hash: TokenHash, user: User) {
        self.users
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(token_hash, user);
    }

    fn read(&self, token_hash: &TokenHash) -> Option<User> {
        self.users
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(token_hash)
            .cloned()
    }

    fn delete(&self, token_hash: &TokenHash) {
        self.users
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(token_hash);
    }
}
