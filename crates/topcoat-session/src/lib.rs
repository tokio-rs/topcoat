mod config;
#[cfg(feature = "router")]
mod router;
mod token;

pub use config::*;
#[cfg(feature = "router")]
pub use router::*;
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
