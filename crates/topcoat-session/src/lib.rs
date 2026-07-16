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
    let hash = token.hash();
    token_store(cx).write(cx, token);
    hash
}

pub fn stop(cx: &Cx) {
    token_store(cx).delete(cx);
}
