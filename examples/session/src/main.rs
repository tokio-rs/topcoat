mod login;
mod logout;

use serde::Deserialize;
use std::{collections::HashMap, sync::{Mutex, PoisonError}, time::SystemTime};
use topcoat::{Result, context::{Cx, app_context}, router::{Slot, href, layout, module_router, page}, session::{SessionConfig, TokenHash, self}, view::{View, view}};

#[tokio::main]
async fn main() {
    // Topcoat issues and carries the session token; where the session records
    // live is up to the application, here the in-memory `Database` below.
    topcoat::start(
        module_router!()
            .cookies()
            .sessions(SessionConfig::default())
            .app_context(Database::default())
            .build(),
    )
    .await
    .unwrap();
}

#[layout]
async fn root(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Sessions"</title>
                topcoat::dev::script()
            </head>
            <body>(slot)</body>
        </html>
    })
}

#[page]
async fn page(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        if let Some(user) = current_user(cx).await? {
            <div>
                "currently logged in as: "
                (&user.name)
            </div>

            <form method="POST" action=(href!(crate::logout::logout))>
                <button>"log out"</button>
            </form>
        } else {
            <div>"currently not logged in"</div>

            <form method="POST" action=(href!(crate::login::login))>
                <input name="name" placeholder="Username" required="true">
                <button>"log in"</button>
            </form>
        }
    })
}

// --- API routes -------------------------------------------------------------

#[derive(Deserialize)]
struct LoginForm {
    name: String,
}

// --- In-memory demo database ------------------------------------------------

#[derive(Debug, Clone)]
struct User {
    name: String,
}

fn db(cx: &Cx) -> &Database {
    app_context(cx)
}

// The session itself only carries a token hash; the user comes from the store.
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
