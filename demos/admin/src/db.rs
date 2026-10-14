use std::time::Duration;

use toasty::{Db, migration::MigrationSet};
use toasty_cli::{Config, MigrationConfig, ToastyCli};
use topcoat::context::{Cx, app_context};

/// Use the package directory so the database path does not depend on where the app starts.
const DATABASE_URL: &str = concat!("sqlite:", env!("CARGO_MANIFEST_DIR"), "/data.db");

const MIGRATIONS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/toasty");

const LATENCY: Duration = Duration::from_millis(500);

const MIGRATIONS: MigrationSet = toasty::embed_migrations!();

pub async fn connect() -> toasty::Result<Db> {
    Db::builder()
        .models(toasty::models!(crate::*))
        .connect(DATABASE_URL)
        .await
}

pub async fn migrate(db: &Db) -> toasty::Result<()> {
    MIGRATIONS.apply(db).await?;
    Ok(())
}

pub fn cli(db: Db) -> ToastyCli {
    let config = Config::new().migration(MigrationConfig::new().path(MIGRATIONS_DIR));
    ToastyCli::with_config(db, config)
}

pub fn db(cx: &Cx) -> Db {
    app_context::<Db>(cx).clone()
}

/// Adds a short delay so the demo's loading placeholders stay visible.
/// Call before queries whose results are shown with a loading state.
pub async fn latency() {
    tokio::time::sleep(LATENCY).await;
}

/// Builds a SQL LIKE pattern for a literal substring search.
/// Use with `like_with_escape` and a backslash escape character. SQLite LIKE
/// ignores case for ASCII letters.
pub fn contains_pattern(text: &str) -> String {
    let mut pattern = String::with_capacity(text.len() + 2);
    pattern.push('%');
    for char in text.chars() {
        if matches!(char, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(char);
    }
    pattern.push('%');
    pattern
}
