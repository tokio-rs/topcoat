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
        Router, RouterBuilderDiscoverExt,
        content::Form,
        error::{SeeOther, see_other},
        layout, page, route,
    },
    session::{self, RouterBuilderSessionExt, SessionConfig, TokenHash},
    view::view,
};

#[tokio::main]
async fn main() {
    // Enable cookies and sessions, register the in-memory database,
    // discover the routes, and start the server.
    // The application is available at http://127.0.0.1:3000 by default.
    topcoat::start(
        Router::builder()
            .cookies()
            .sessions(SessionConfig::default())
            .app_context(Database::default())
            .discover()
            .build(),
    )
    .await
    .unwrap();
}

#[layout("/")]
async fn root(slot: Result) -> Result {
    // Wrap every page in a complete HTML document.
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
    // Read the current user from the active session, when one exists.
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

// --- API routes -------------------------------------------------------------

#[derive(Deserialize)]
struct LoginForm {
    name: String,
}

#[route(POST "/login")]
async fn login(cx: &Cx, Form(form): Form<LoginForm>) -> Result<SeeOther> {
    // A real application would verify credentials before starting the session.
    let session = session::start(cx).await?;

    // Associate the new session with the submitted user.
    db(cx).create(session, User { name: form.name });

    // Redirect the browser back to the home page.
    Ok(see_other("/"))
}

#[route(POST "/logout")]
async fn logout(cx: &Cx) -> Result<SeeOther> {
    // Stop the current session and delete its record from the database.
    if let Some(token_hash) = session::stop(cx).await? {
        db(cx).delete(&token_hash);
    }

    Ok(see_other("/"))
}

// --- In-memory demo database ------------------------------------------------

#[derive(Debug, Clone)]
struct User {
    name: String,
}

// Retrieve the database registered as application context.
fn db(cx: &Cx) -> &Database {
    app_context(cx)
}

// Resolve the current session token to a user.
async fn current_user(cx: &Cx) -> Result<Option<User>> {
    let Some(token_hash) = session::token_hash(cx).await? else {
        return Ok(None);
    };

    Ok(db(cx).read(&token_hash))
}

/// A persisted session record containing the authenticated user and expiry.
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
            // Ignore expired sessions.
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
