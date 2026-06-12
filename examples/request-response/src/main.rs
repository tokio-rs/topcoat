use axum::body::to_bytes;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, Form, FromRequest, IntoResponse, Json, Response, Router, bad_request, headers, route,
    },
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::new().discover()).await.unwrap();
}

// --- JSON requests and responses -------------------------------------------

#[derive(Deserialize, Serialize)]
struct User {
    name: String,
}

// Json<T> parses an application/json request body and serializes the response.
#[route(POST "/api/users")]
async fn create_user(Json(user): Json<User>) -> Result<Json<User>> {
    Ok(Json(user))
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

// For GET and HEAD requests, Form<T> reads URL-encoded values from the query string.
#[route(GET "/api/search")]
async fn search(Form(input): Form<Search>) -> Result<Json<SearchResult>> {
    Ok(Json(SearchResult {
        query: input.q,
        limit: input.limit.unwrap_or(10),
    }))
}

// --- Form request and response bodies --------------------------------------

// For other methods, Form<T> reads and writes application/x-www-form-urlencoded bodies.
#[route(POST "/api/form-echo")]
async fn form_echo(Form(input): Form<Search>) -> Result<Form<Search>> {
    Ok(Form(input))
}

// --- Raw request bodies -----------------------------------------------------

// Body gives the handler the raw stream when it wants to parse bytes itself.
#[route(POST "/api/upload")]
async fn upload(body: Body) -> Result<String> {
    let bytes = to_bytes(body, usize::MAX)
        .await
        .map_err(|error| bad_request(format!("failed to read request body: {error}")))?;

    Ok(format!("received {} bytes", bytes.len()))
}

// --- Custom responses -------------------------------------------------------

struct Csv(String);

impl IntoResponse for Csv {
    fn into_response(self) -> Result<Response> {
        Ok(Response::builder()
            .header("Content-Type", "text/csv; charset=utf-8")
            .body(Body::from(self.0))?)
    }
}

// Returning a custom IntoResponse type lets the handler choose headers and body.
#[route(GET "/api/report.csv")]
async fn report() -> Result<Csv> {
    Ok(Csv("name,total\nAda,42\nGrace,64\n".to_string()))
}

// --- Custom request extractors ---------------------------------------------

// SignedJson<T> is an example extractor that validates a header before parsing JSON.
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

        let bytes = to_bytes(body, usize::MAX)
            .await
            .map_err(|error| bad_request(format!("failed to read request body: {error}")))?;

        Ok(Self(serde_json::from_slice(&bytes)?))
    }
}

#[route(POST "/api/signed")]
async fn signed(SignedJson(user): SignedJson<User>) -> Result<Json<User>> {
    Ok(Json(user))
}
