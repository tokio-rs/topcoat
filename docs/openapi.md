Use [OpenAPI](https://www.openapis.org) to describe your HTTP API. Documentation tools can display this description, and client generators can use it to create code that calls your API. In Topcoat, you describe routes with [`Operation`] and use [`OpenApi`] to collect those descriptions into a [`Document`].

Enable Topcoat's `openapi` feature. Also add [schemars](https://docs.rs/schemars), which describes the data in your request and response bodies:

```sh
cargo add schemars
```

# Documenting a route

Add an `Operation` to a route using `with(...)`. Use [`request`](Operation::request) to describe the request body and [`response_with`](Operation::response_with) to describe a response that has a body. Derive `JsonSchema` on the data types used in these bodies:

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

Topcoat gets the path, HTTP methods, and path parameters from the router. You provide the other details in the operation. Topcoat does not check that these details match the handler, so keep them up to date when you change what the handler accepts or returns.

For a response with no body, use [`response`](Operation::response). Doc comments on your data types and their fields become descriptions in the generated schema.

# Serving the document

Set the API's title, version, and other document settings with [`OpenApi`]. Pass your router to [`generate`](OpenApi::generate) to create a [`Document`]. Return the document from a route to serve it as JSON:

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

[`router(cx)`](crate::router::router) gives you the router handling the current request. In this example, the document route has no operation of its own. It stays out of the document unless it inherits an operation, as described below.

You can also generate the document outside a request. For example, a test or an application command can build the router, call `generate`, and save the result with [`to_json_pretty`](Document::to_json_pretty):

```rust
use topcoat::{openapi::OpenApi, router::Router};

fn export(router: &Router) -> std::io::Result<()> {
    let document = OpenApi::new("Acme API", "1.0.0").generate(router);
    std::fs::write("openapi.json", document.to_json_pretty())
}
```

# Inherited settings

Add an `Operation` to a [layer](macro@crate::router::layer) to set documentation defaults for every route under its path. Each route inherits these defaults and is included in the document. Use `.hidden(true)` to leave a route out:

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

A route can add its own operation to override or extend these defaults. For example, a summary on the route overrides the summary from its layer.

Tags, parameters, and responses are combined. The route keeps the entries from its layers. If it defines the same parameter or a response with the same status code, its definition replaces the inherited one.

You can also put an operation on a layout to document the pages it wraps, or on the router builder with `.with(...)` to document every route. These settings follow the rules of [route context](crate::context#route-context).

# Parameters

Topcoat automatically includes path parameters in the document and describes them as strings. Add a [`Parameter`] when you want to specify a different type or add a description:

```rust
use topcoat::openapi::{Operation, Parameter};

let operation = Operation::new().parameter(
    Parameter::path("user_id")
        .schema::<u64>()
        .description("The ID of the user."),
);
```

Use `Parameter` to describe individual query parameters, headers, or cookies too. These are optional by default. Call [`required(true)`](Parameter::required) to mark one as required.

To document a struct used with [`query_params`](macro@crate::router::query_params), derive `JsonSchema` and pass the struct's type to [`query`](Operation::query). Topcoat adds a query parameter for each field. An `Option` field is optional, and each field's doc comment becomes its description:

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

The [`Content`] trait describes a body's format, called its media type, and the shape of its data, called its schema. JSON and form extractors implement `Content` when their data type implements `JsonSchema`. String bodies are described as plain text.

To document an optional request body, wrap the body type in `Option`. For example, `.request::<Option<Json<CreateUser>>>()` says that the client may leave out the JSON body.

For a custom extractor or response type, implement `Content` to describe its body:

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

Schemas follow Serde's rules for reading data into a type, even when the type is used in a response. If a type uses different rules for reading and writing, the response schema may differ from the data sent to the client. For example, `#[serde(skip_serializing)]` leaves a field out when writing a response, but that field can still appear in the schema.

# Authentication

Give each authentication method a name and register it as a [`SecurityScheme`]. Use that name in [`security`](Operation::security) to document which method a route requires. Put this setting on a layer to apply it to every route below the layer. A route can use `.no_security()` to document that it does not require authentication:

```rust
use topcoat::openapi::{OpenApi, Operation, SecurityScheme};

let api = OpenApi::new("Acme API", "1.0.0")
    .security_scheme("token", SecurityScheme::bearer().bearer_format("JWT"));

// On a layer: every route below requires the token.
let protected = Operation::new().security("token");
// On a route below that layer: usable without the token.
let public = Operation::new().no_security();
```

Call `security` more than once on an operation to list alternative authentication methods. Clients can use any one of them. When an operation sets its own security requirements, they replace all the requirements inherited from its layers.

These settings describe authentication in the document. Your layers and handlers must still check credentials and enforce access rules.
