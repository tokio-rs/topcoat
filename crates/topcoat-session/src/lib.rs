mod config;
mod token;

pub use config::*;
pub use token::*;

use topcoat_core::context::Cx;

pub fn start(cx: &Cx) -> TokenHash {
    let token = Token::random();
    token.hash()
}

pub fn stop(cx: &Cx) {}

pub fn session_token(cx: &Cx) -> Option<TokenHash> {
    None
}
