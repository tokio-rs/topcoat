use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
    time::SystemTime,
};

use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    cookie::RouterBuilderCookieExt,
    router::{
        Form, Router, RouterBuilderDiscoverExt,
        error::{SeeOther, see_other},
        layout, page, route,
    },
    session::{self, RouterBuilderSessionExt, TokenHash},
    view::view,
};

#[tokio::main]
async fn main() {
    topcoat::start(
        Router::builder()
            .cookies()
            .sessions(session::Config::default())
            .app_context(Database::default())
            .discover()
            .build(),
    )
    .await
    .unwrap();
}

#[layout("/")]
async fn root(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Sessions"</title>
                topcoat::dev::script()
            </head>
            <body>(slot?)</body>
        </html>
    }
}

#[page("/")]
async fn page(cx: &Cx) -> Result {
    view! {
        if let Some(user) = current_user(cx).await? {
            <div>
                "currently logged in as: "
                (&user.name)
            </div>
            <form method="POST" action="/logout"><button>"log out"</button></form>
        } else {
            <div>"currently not logged in"</div>
            <form method="POST" action="/login">
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
async fn login(cx: &Cx, Form(form): Form<LoginForm>) -> Result<SeeOther> {
    // A real application would verify credentials here before starting the session.
    let session = session::start(cx).await?;
    db(cx).create(session, User { name: form.name });
    Ok(see_other("/"))
}

#[route(POST "/logout")]
async fn logout(cx: &Cx) -> Result<SeeOther> {
    if let Some(token_hash) = session::stop(cx).await? {
        db(cx).delete(&token_hash);
    }
    Ok(see_other("/"))
}

// -----------------------
// In-memory demo database

#[derive(Debug, Clone)]
struct User {
    name: String,
}

fn db(cx: &Cx) -> &Database {
    app_context(cx)
}

async fn current_user(cx: &Cx) -> Result<Option<User>> {
    let Some(token_hash) = session::token_hash(cx).await? else {
        return Ok(None);
    };
    Ok(db(cx).read(&token_hash))
}

/// A session record as the application persists it: the user the session
/// authenticates plus its expiry.
#[derive(Debug)]
struct Record {
    user: User,
    expires_at: SystemTime,
}

#[derive(Debug, Default)]
struct Database {
    sessions: Mutex<HashMap<TokenHash, Record>>,
}

impl Database {
    fn create(&self, session: session::Session, user: User) {
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                session.token_hash,
                Record {
                    user,
                    expires_at: session.expires_at,
                },
            );
    }

    fn read(&self, token_hash: &TokenHash) -> Option<User> {
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(token_hash)
            .filter(|record| record.expires_at > SystemTime::now())
            .map(|record| record.user.clone())
    }

    fn delete(&self, token_hash: &TokenHash) {
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(token_hash);
    }
}
