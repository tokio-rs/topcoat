An [OpenAPI](https://www.openapis.org) document describes an HTTP API in a format that documentation tools and client generators understand. Attach an [`Operation`] to the routes that belong to your API, then create a [`Document`] that lists them with [`OpenApi`].

Enable the `openapi` feature to use this module. Request and response schemas come from [schemars](https://docs.rs/schemars), so add it to your dependencies as well:

```sh
cargo add schemars
```

# Documenting a route

Add an `Operation` to a route with `with(...)`. Name the request body with [`request`](Operation::request) and each response with [`response_with`](Operation::response_with), and derive `JsonSchema` on the types they carry:

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::Cx,
    openapi::Operation,
    router::{StatusCode, content::Json, route},
};

#[derive(Deserialize, JsonSchema)]
struct CreateUser {
    /// The name to show for the user.
    name: String,
}

#[derive(Serialize, JsonSchema)]
struct User {
    id: u64,
    name: String,
}

#[route(POST "/api/users", with(
    Operation::new()
        .summary("Create a user")
        .request::<Json<CreateUser>>()
        .response_with::<Json<User>>(StatusCode::CREATED, "The new user.")
        .response(StatusCode::CONFLICT, "The name is taken.")
))]
async fn create_user(cx: &Cx, Json(input): Json<CreateUser>) -> Result<(StatusCode, Json<User>)> {
    let user = User { id: 1, name: input.name };
    Ok((StatusCode::CREATED, Json(user)))
}
```

The document takes the route's path, methods, and path parameters from the router. Everything else comes from the operation. An operation is not checked against the handler, so update it when the handler's body or return type changes.

Use [`response`](Operation::response) for a response without a body. Doc comments on a type and its fields become descriptions in its schema.

# Serving the document

[`OpenApi`] holds the settings of the document, such as the API's title and version. Call [`generate`](OpenApi::generate) with a router to create the [`Document`]. A route can return it to serve it as JSON:

```rust
use topcoat::{
    Result,
    context::Cx,
    openapi::{Document, OpenApi},
    router::{route, router},
};

#[route(GET "/openapi.json")]
async fn spec(cx: &Cx) -> Result<Document> {
    Ok(OpenApi::new("Acme API", "1.0.0")
        .description("Everything you need for tea.")
        .server("https://api.example.com")
        .generate(&router(cx)))
}
```

[`router(cx)`](crate::router::router) returns the router that handles the request. This route has no operation, so the document does not list it.

`generate` only needs a router, not a request. A test or a command of your application can build the router and write the document to a file with [`to_json_pretty`](Document::to_json_pretty):

```rust
use topcoat::{openapi::OpenApi, router::Router};

fn export(router: &Router) -> std::io::Result<()> {
    let document = OpenApi::new("Acme API", "1.0.0").generate(router);
    std::fs::write("openapi.json", document.to_json_pretty())
}
```

# Documenting a group of routes

A route is documented when an `Operation` is attached to it or to something that wraps it. Add one to a [layer](macro@crate::router::layer) to document every route under its path:

```rust
use topcoat::{
    Result,
    context::Cx,
    openapi::Operation,
    router::{Body, Next, StatusCode, layer, response::Response, route},
};

#[layer("/api", with(
    Operation::new()
        .tag("api")
        .response(StatusCode::UNAUTHORIZED, "The token is missing or wrong.")
))]
async fn api(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    next.run(cx, body).await
}

// Documented through the layer, with its tag and 401 response.
#[route(GET "/api/health")]
async fn health(cx: &Cx) -> Result<&'static str> {
    Ok("ok")
}

// Left out of the document.
#[route(GET "/api/debug", with(Operation::new().hidden(true)))]
async fn debug(cx: &Cx) -> Result<&'static str> {
    Ok("debug")
}
```

A route combines its own operation with those of its layers. For a setting such as the summary, the closest value takes priority, so a route overrides its layers. Tags, parameters, and responses add up instead. A route keeps the ones its layers declare and replaces only a parameter or response status that it declares itself.

Operations follow the rules of [route context](crate::context#route-context). An operation on a layout documents the pages it wraps, and one added to the router builder with `.with(...)` documents every route.

# Parameters

Path parameters are part of a route's path, so the document lists them without an entry. They are documented as strings. Add a [`Parameter`] to give one a type or a description:

```rust
use topcoat::openapi::{Operation, Parameter};

let operation = Operation::new().parameter(
    Parameter::path("user_id")
        .schema::<u64>()
        .description("The ID of the user."),
);
```

`Parameter` also describes single query keys, headers, and cookies. These are optional unless marked with [`required`](Parameter::required).

For a struct read with [`query_params`](macro@crate::router::query_params), derive `JsonSchema` and pass it to [`query`](Operation::query). Each field becomes a query parameter. Fields of type `Option` are optional, and doc comments become descriptions:

```rust
use schemars::JsonSchema;
use topcoat::{openapi::Operation, router::query_params};

#[query_params]
#[derive(JsonSchema)]
struct PostsQuery {
    /// The text to search for.
    q: String,
    /// The page to show, starting at 1.
    page: Option<u32>,
}

let operation = Operation::new().query::<PostsQuery>();
```

# Request and response bodies

A type that implements [`Content`] can describe a body. It names a media type and a schema. The JSON and form extractors implement it for payloads that implement `JsonSchema`, and strings are documented as plain text.

Wrap a request type in `Option` for a body that requests may leave out, such as `.request::<Option<Json<CreateUser>>>()`.

Implement `Content` for your own extractors and response types:

```rust
use topcoat::openapi::{
    Content,
    schemars::{Schema, SchemaGenerator, json_schema},
};

struct Csv(String);

impl Content for Csv {
    const MEDIA_TYPE: &'static str = "text/csv";

    fn schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({ "type": "string" })
    }
}
```

Schemas describe a type as it is read from a request. Serde attributes are respected. A type that is written differently than it is read, for example because of `#[serde(skip_serializing)]`, is still documented in its read form when used in a response.

# Authentication

Register each way to authenticate as a [`SecurityScheme`] under a name, then require it by that name with [`security`](Operation::security). A layer is a good place for the requirement, because it also covers every route below it:

```rust
use topcoat::openapi::{OpenApi, Operation, SecurityScheme};

let api = OpenApi::new("Acme API", "1.0.0")
    .security_scheme("token", SecurityScheme::bearer().bearer_format("JWT"));

// On a layer: every route below requires the token.
let protected = Operation::new().security("token");
// On a route below that layer: usable without the token.
let public = Operation::new().no_security();
```

Calling `security` several times on one operation lists alternatives, so clients may use any of the schemes. The requirements of an operation replace those of its layers as a whole.

The security settings only document the API. Checking credentials is still up to your layers and handlers.

# Mistakes found while generating

`generate` panics when the routes are documented inconsistently. This happens when an operation describes a path parameter that its route's path does not have, requires a security scheme that is not registered, or shares its operation ID with another operation. Generate the document in a test to catch these mistakes before a request does.
