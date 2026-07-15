mod config;
mod token;

pub use config::*;
pub use token::*;

use topcoat_core::context::Cx;

pub fn start_session(cx: &Cx) -> SessionTokenHash {
    let token = SessionToken::random();
    token.hash()
}

pub fn stop_session(cx: &Cx) {}

pub fn session_token(cx: &Cx) -> Option<SessionTokenHash> {
    None
}
