mod api;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body,
        error::bad_request,
        module_router,
        request::{Bytes, FromRequest, headers},
        response::{IntoResponse, Response},
    },
};

#[tokio::main]
async fn main() {
    topcoat::start(module_router!().build()).await.unwrap();
}

// --- JSON requests and responses -------------------------------------------

#[derive(Deserialize, Serialize)]
struct User {
    name: String,
}

// --- Query-string form parsing ---------------------------------------------

#[derive(Deserialize, Serialize)]
struct Search {
    q: String,
    limit: Option<u8>,
}

#[derive(Serialize)]
struct SearchResult {
    query: String,
    limit: u8,
}

// --- Custom responses -------------------------------------------------------

struct Csv(String);

impl IntoResponse for Csv {
    fn into_response(self, _cx: &Cx) -> Result<Response> {
        Ok(Response::builder()
            .header("Content-Type", "text/csv; charset=utf-8")
            .body(Body::from(self.0))?)
    }
}

// --- Custom request parsing -------------------------------------------------

// SignedJson<T> is an example parser that validates a header before parsing JSON.
struct SignedJson<T>(T);

impl<T> FromRequest for SignedJson<T>
where
    T: DeserializeOwned,
{
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        let signature = headers(cx)
            .get("x-signature")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| bad_request("missing x-signature header"))?;

        if signature != "topcoat-demo" {
            return Err(bad_request("invalid x-signature header").into());
        }

        // Delegating the buffering to Bytes keeps the body limit applied.
        let bytes = Bytes::from_request(cx, body).await?;

        Ok(Self(serde_json::from_slice(&bytes)?))
    }
}
