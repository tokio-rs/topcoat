use leptos::server_fn::{
    codec::JsonEncoding,
    error::{FromServerFnError, ServerFnErrorErr},
};
use serde::{Deserialize, Serialize};

/// Why a page cannot be shown.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum PageError {
    /// The page does not exist, rendered as the 404 page.
    NotFound,
    /// The request is malformed, rendered as the 422 page.
    Invalid,
    /// Loading the page data failed, for example because the network is down.
    Server(ServerFnErrorErr),
}

impl FromServerFnError for PageError {
    type Encoder = JsonEncoding;

    fn from_server_fn_error(error: ServerFnErrorErr) -> Self {
        Self::Server(error)
    }
}
