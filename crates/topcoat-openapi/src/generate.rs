use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
};

use schemars::{Schema, SchemaGenerator, generate::SchemaSettings, json_schema};
use serde_json::Value;
use topcoat_router::{Method, Methods, RegisteredRoute, Router};

use crate::{
    Document, OpenApi, Operation, Parameter,
    content::{ContentType, SchemaFn},
    document::{
        Components, Info, MediaType, OperationObject, ParameterObject, PathItem, RequestBodyObject,
        ResponseObject, SecurityRequirement, Server, Tag,
    },
    operation::QueryType,
    parameter::ParameterLocation,
};

/// The OpenAPI version the generated documents follow.
const OPENAPI_VERSION: &str = "3.1.0";

/// Creates the document for the documented routes of `router`.
pub(crate) fn generate(api: &OpenApi, router: &Router) -> Document {
    let mut generation = Generation::new(api);

    // A route for a specific method takes precedence over an any-method
    // route at its path. Any-method routes therefore come last and fill the
    // methods that are still free.
    let mut any_method = Vec::new();
    for route in router.routes() {
        let levels = route
            .context()
            .iter::<Operation>()
            .chain(router.context().iter::<Operation>());
        let Some(operation) = Operation::combine(levels) else {
            continue;
        };
        if operation.hidden == Some(true) {
            continue;
        }
        match route.methods() {
            Methods::Only(methods) => generation.document(&route, &operation, methods, false),
            Methods::Any => any_method.push((route, operation)),
        }
    }
    for (route, operation) in any_method {
        generation.document(&route, &operation, &PathItem::METHODS, true);
    }

    generation.finish()
}

/// A document under construction.
struct Generation<'api> {
    api: &'api OpenApi,
    schemas: SchemaGenerator,
    paths: BTreeMap<String, PathItem>,
    /// The operation IDs in use, each with the operation that uses it.
    operation_ids: HashMap<Cow<'static, str>, String>,
}

impl<'api> Generation<'api> {
    /// Starts an empty document with the settings of `api`.
    fn new(api: &'api OpenApi) -> Self {
        let settings = SchemaSettings::draft2020_12().with(|settings| {
            settings.definitions_path = "/components/schemas".into();
            settings.meta_schema = None;
        });
        Self {
            api,
            schemas: settings.into_generator(),
            paths: BTreeMap::new(),
            operation_ids: HashMap::new(),
        }
    }

    /// Documents `route` under each of `methods` that a document can
    /// describe. With `only_free`, methods that already have an operation at
    /// the route's path are left alone.
    fn document(
        &mut self,
        route: &RegisteredRoute<'_>,
        operation: &Operation,
        methods: &[Method],
        only_free: bool,
    ) {
        // A catch-all parameter is documented like a regular one.
        let path = route.path().as_str().replace("{*", "{");
        let mut object = None;
        for method in methods {
            if !PathItem::describes(method) {
                continue;
            }
            let free = self
                .paths
                .get_mut(&path)
                .and_then(|item| item.operation_mut(method))
                .is_none_or(|documented| documented.is_none());
            if only_free && !free {
                continue;
            }

            if let Some(id) = &operation.id {
                let used_by = format!("{method} {path}");
                if let Some(first) = self.operation_ids.insert(id.clone(), used_by) {
                    panic!(
                        "the operation ID `{id}` is used by both `{first}` and `{method} {path}`"
                    );
                }
            }

            let object = object
                .get_or_insert_with(|| self.operation_object(&path, route, operation))
                .clone();
            let item = self.paths.entry(path.clone()).or_default();
            if let Some(documented) = item.operation_mut(method) {
                *documented = Some(object);
            }
        }
    }

    /// Completes the document.
    fn finish(mut self) -> Document {
        let api = self.api;
        Document {
            openapi: OPENAPI_VERSION,
            info: Info {
                title: api.title.clone(),
                description: api.description.clone(),
                version: api.version.clone(),
            },
            servers: api
                .servers
                .iter()
                .map(|url| Server { url: url.clone() })
                .collect(),
            tags: api
                .tags
                .iter()
                .map(|(name, description)| Tag {
                    name: name.clone(),
                    description: description.clone(),
                })
                .collect(),
            paths: self.paths,
            components: Components {
                schemas: self.schemas.take_definitions(true).into_iter().collect(),
                security_schemes: api.security_schemes.iter().cloned().collect(),
            },
        }
    }

    /// Describes `operation` as served by `route` at `path`.
    fn operation_object(
        &mut self,
        path: &str,
        route: &RegisteredRoute<'_>,
        operation: &Operation,
    ) -> OperationObject {
        OperationObject {
            tags: operation.tags.clone(),
            summary: operation.summary.clone(),
            description: operation.description.clone(),
            operation_id: operation.id.clone(),
            parameters: self.parameters(path, route, operation),
            request_body: operation.request.map(|content| RequestBodyObject {
                required: content.required,
                content: self.content(content),
            }),
            responses: operation
                .responses
                .iter()
                .map(|(status, response)| {
                    let response = ResponseObject {
                        description: response.description.clone(),
                        content: response
                            .content
                            .map_or_default(|content| self.content(content)),
                    };
                    (*status, response)
                })
                .collect(),
            deprecated: operation.deprecated == Some(true),
            security: operation
                .security
                .as_ref()
                .map(|schemes| self.security(path, schemes)),
        }
    }

    /// Lists the parameters of `operation`: the parameters of the route's
    /// path first, then the ones the operation describes, then the fields
    /// of its query structs.
    fn parameters(
        &mut self,
        path: &str,
        route: &RegisteredRoute<'_>,
        operation: &Operation,
    ) -> Vec<ParameterObject> {
        let mut parameters = Vec::new();
        let mut described: Vec<&Parameter> = operation.parameters.iter().collect();

        for name in route
            .path()
            .segments()
            .filter_map(|segment| segment.param_name())
        {
            let described = described
                .iter()
                .position(|parameter| {
                    parameter.location == ParameterLocation::Path && parameter.name == name
                })
                .map(|position| described.remove(position));
            parameters.push(ParameterObject {
                name: Cow::Owned(name.to_owned()),
                location: ParameterLocation::Path,
                description: described.and_then(|parameter| parameter.description.clone()),
                required: true,
                schema: self.parameter_schema(described.and_then(|parameter| parameter.schema)),
            });
        }

        for parameter in described {
            assert!(
                parameter.location != ParameterLocation::Path,
                "the operation at `{path}` describes the path parameter `{}`, which its path \
                 does not have",
                parameter.name,
            );
            parameters.push(ParameterObject {
                name: parameter.name.clone(),
                location: parameter.location,
                description: parameter.description.clone(),
                required: parameter.required,
                schema: self.parameter_schema(parameter.schema),
            });
        }

        // A parameter the operation describes itself takes priority over a
        // field of the same name.
        for query in &operation.queries {
            for parameter in self.query_parameters(query) {
                let described = parameters.iter().any(|other| {
                    other.location == ParameterLocation::Query && other.name == parameter.name
                });
                if !described {
                    parameters.push(parameter);
                }
            }
        }

        parameters
    }

    /// Returns the schema of a parameter's value, which is a string unless
    /// the parameter names another type.
    fn parameter_schema(&mut self, schema: Option<SchemaFn>) -> Schema {
        match schema {
            Some(schema) => schema(&mut self.schemas),
            None => json_schema!({ "type": "string" }),
        }
    }

    /// Describes each field of a query struct as a query parameter.
    fn query_parameters(&mut self, query: &QueryType) -> Vec<ParameterObject> {
        let mut schema = (query.schema)(&mut self.schemas);
        let is_object = schema.get("type").is_some_and(|ty| ty == "object");
        let properties = match schema.remove("properties") {
            Some(Value::Object(properties)) => properties,
            None if is_object => serde_json::Map::new(),
            _ => panic!(
                "`{}` cannot be documented as query parameters, because it is not a struct \
                 with named fields",
                query.name,
            ),
        };
        let required: Vec<&str> = schema
            .get("required")
            .and_then(Value::as_array)
            .map_or_default(|names| names.iter().filter_map(Value::as_str).collect());

        properties
            .into_iter()
            .filter_map(|(name, property)| {
                let mut schema = Schema::try_from(property).ok()?;
                let description = match schema.remove("description") {
                    Some(Value::String(description)) => Some(Cow::Owned(description)),
                    _ => None,
                };
                let required = required.contains(&name.as_str());
                if !required {
                    remove_null(&mut schema);
                }
                Some(ParameterObject {
                    name: Cow::Owned(name),
                    location: ParameterLocation::Query,
                    description,
                    required,
                    schema,
                })
            })
            .collect()
    }

    /// Describes a body by its media type and schema.
    fn content(&mut self, content: ContentType) -> BTreeMap<&'static str, MediaType> {
        let schema = (content.schema)(&mut self.schemas);
        BTreeMap::from([(content.media_type, MediaType { schema })])
    }

    /// Lists the security schemes that the operation at `path` accepts.
    fn security(&self, path: &str, schemes: &[Cow<'static, str>]) -> Vec<SecurityRequirement> {
        schemes
            .iter()
            .map(|scheme| {
                assert!(
                    self.api
                        .security_schemes
                        .iter()
                        .any(|(name, _)| name == scheme),
                    "the operation at `{path}` requires the security scheme `{scheme}`, which \
                     is not registered",
                );
                BTreeMap::from([(scheme.clone(), Vec::new())])
            })
            .collect()
    }
}

/// Removes `null` from the values that `schema` accepts.
///
/// An optional field of a query struct accepts `null`, but a request leaves
/// an optional query parameter out instead.
fn remove_null(schema: &mut Schema) {
    let Some(object) = schema.as_object_mut() else {
        return;
    };

    if let Some(Value::Array(types)) = object.get_mut("type") {
        types.retain(|ty| ty != "null");
        if types.len() == 1 {
            let only = types.remove(0);
            object.insert("type".to_owned(), only);
        }
    }

    if let Some(Value::Array(options)) = object.get_mut("anyOf") {
        options.retain(|option| option.get("type").is_none_or(|ty| ty != "null"));
        if let [Value::Object(only)] = options.as_mut_slice() {
            let only = std::mem::take(only);
            object.remove("anyOf");
            object.extend(only);
        }
    }
}

#[cfg(test)]
mod tests {
    use schemars::JsonSchema;
    use serde_json::json;
    use topcoat_core::context::Cx;
    use topcoat_router::{
        Body, LayerFn, LayerFuture, Next, Path, RouteFn, RouteFuture, RouterBuilder, StatusCode,
        content::Json, response::IntoResponse,
    };

    use super::*;
    use crate::SecurityScheme;

    // -- Test helpers --

    fn handler(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { "ok".into_response(cx) })
    }

    fn pass_through<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        next.run(cx, body)
    }

    /// A route answering `method` at `path`.
    fn route(method: Method, path: &'static str) -> RouteFn {
        RouteFn::new(method, path, handler)
    }

    /// A layer wrapping the routes under `path`.
    fn layer(path: &'static str) -> LayerFn {
        LayerFn::new(Some(Path::new(path)), pass_through)
    }

    /// Generates the document of `router` with default settings, as JSON.
    fn document(router: RouterBuilder) -> Value {
        document_with(&OpenApi::new("Test", "1.0.0"), router)
    }

    /// Generates the document of `router` with the settings of `api`, as JSON.
    fn document_with(api: &OpenApi, router: RouterBuilder) -> Value {
        serde_json::to_value(api.generate(&router.build())).unwrap()
    }

    /// The documented operations, as `method path`, in document order.
    fn operations(document: &Value) -> Vec<String> {
        document["paths"]
            .as_object()
            .unwrap()
            .iter()
            .flat_map(|(path, item)| {
                item.as_object()
                    .unwrap()
                    .keys()
                    .map(move |method| format!("{method} {path}"))
            })
            .collect()
    }

    /// The parameters of an operation, as `(name, location, required)`.
    fn parameters(operation: &Value) -> Vec<(&str, &str, bool)> {
        operation["parameters"]
            .as_array()
            .map_or_default(|parameters| {
                parameters
                    .iter()
                    .map(|parameter| {
                        (
                            parameter["name"].as_str().unwrap(),
                            parameter["in"].as_str().unwrap(),
                            parameter["required"].as_bool().unwrap_or(false),
                        )
                    })
                    .collect()
            })
    }

    /// Follows a `$ref` into the document, so tests do not depend on where a
    /// schema is listed.
    fn resolve<'a>(document: &'a Value, schema: &'a Value) -> &'a Value {
        match schema.get("$ref").and_then(Value::as_str) {
            Some(reference) => document
                .pointer(reference.strip_prefix('#').unwrap())
                .unwrap(),
            None => schema,
        }
    }

    #[derive(JsonSchema)]
    #[allow(dead_code)]
    struct CreateUser {
        name: String,
    }

    #[derive(JsonSchema)]
    #[allow(dead_code)]
    struct User {
        id: u64,
        name: String,
    }

    // -- Which routes are documented --

    #[test]
    fn only_routes_with_an_operation_are_documented() {
        let document = document(
            RouterBuilder::new()
                .route(route(Method::GET, "/users").with(Operation::new()))
                .route(route(Method::GET, "/internal")),
        );

        assert_eq!(operations(&document), ["get /users"]);
    }

    #[test]
    fn a_layer_documents_the_routes_it_wraps() {
        let document = document(
            RouterBuilder::new()
                .layer(layer("/api").with(Operation::new().tag("api")))
                .route(route(Method::GET, "/api/users"))
                .route(route(Method::POST, "/api/users").with(Operation::new().tag("write")))
                .route(route(Method::GET, "/about")),
        );

        assert_eq!(operations(&document), ["get /api/users", "post /api/users"]);
        assert_eq!(
            document["paths"]["/api/users"]["get"]["tags"],
            json!(["api"])
        );
        assert_eq!(
            document["paths"]["/api/users"]["post"]["tags"],
            json!(["api", "write"])
        );
    }

    #[test]
    fn an_operation_on_the_router_builder_documents_every_route() {
        let document = document(
            RouterBuilder::new()
                .with(Operation::new())
                .route(route(Method::GET, "/users"))
                .route(route(Method::GET, "/about")),
        );

        assert_eq!(operations(&document), ["get /about", "get /users"]);
    }

    #[test]
    fn a_hidden_route_is_left_out() {
        let document = document(
            RouterBuilder::new()
                .layer(layer("/api").with(Operation::new()))
                .route(route(Method::GET, "/api/users"))
                .route(route(Method::GET, "/api/debug").with(Operation::new().hidden(true))),
        );

        assert_eq!(operations(&document), ["get /api/users"]);
    }

    // -- Methods --

    #[test]
    fn a_route_is_documented_under_each_of_its_methods() {
        let methods: &'static [Method] = &[Method::GET, Method::POST];
        let document = document(
            RouterBuilder::new()
                .route(RouteFn::new(methods, "/form", handler).with(Operation::new())),
        );

        assert_eq!(operations(&document), ["get /form", "post /form"]);
    }

    #[test]
    fn an_any_method_route_fills_the_methods_no_other_route_serves() {
        let document = document(
            RouterBuilder::new()
                .route(
                    RouteFn::new(Methods::Any, "/hook", handler)
                        .with(Operation::new().summary("any")),
                )
                .route(route(Method::POST, "/hook").with(Operation::new().summary("post"))),
        );

        let item = &document["paths"]["/hook"];
        assert_eq!(item["post"]["summary"], "post");
        assert_eq!(item["get"]["summary"], "any");
        assert_eq!(item["delete"]["summary"], "any");
    }

    #[test]
    fn a_method_the_document_cannot_describe_is_skipped() {
        let document = document(
            RouterBuilder::new()
                .route(route(Method::CONNECT, "/tunnel").with(Operation::new()))
                .route(route(Method::GET, "/users").with(Operation::new())),
        );

        // The path is left out as a whole rather than listed without
        // operations.
        assert_eq!(
            document["paths"]
                .as_object()
                .unwrap()
                .keys()
                .collect::<Vec<_>>(),
            ["/users"]
        );
    }

    // -- Parameters --

    #[test]
    fn path_parameters_are_documented_without_being_described() {
        let document =
            document(RouterBuilder::new().route(
                route(Method::GET, "/users/{user_id}/posts/{post_id}").with(Operation::new()),
            ));

        let operation = &document["paths"]["/users/{user_id}/posts/{post_id}"]["get"];
        assert_eq!(
            parameters(operation),
            [("user_id", "path", true), ("post_id", "path", true)]
        );
        assert_eq!(operation["parameters"][0]["schema"]["type"], "string");
    }

    #[test]
    fn a_described_path_parameter_keeps_its_place_and_gains_its_details() {
        let document = document(
            RouterBuilder::new().route(
                route(Method::GET, "/users/{user_id}/posts/{post_id}").with(
                    Operation::new().parameter(
                        Parameter::path("post_id")
                            .schema::<u64>()
                            .description("The post."),
                    ),
                ),
            ),
        );

        let operation = &document["paths"]["/users/{user_id}/posts/{post_id}"]["get"];
        assert_eq!(
            parameters(operation),
            [("user_id", "path", true), ("post_id", "path", true)]
        );
        let post_id = &operation["parameters"][1];
        assert_eq!(post_id["description"], "The post.");
        assert_eq!(post_id["schema"]["type"], "integer");
    }

    #[test]
    fn a_catch_all_parameter_is_documented_like_a_regular_one() {
        let document = document(
            RouterBuilder::new().route(route(Method::GET, "/files/{*rest}").with(Operation::new())),
        );

        let operation = &document["paths"]["/files/{rest}"]["get"];
        assert_eq!(parameters(operation), [("rest", "path", true)]);
    }

    #[test]
    #[should_panic(expected = "post_id")]
    fn describing_a_path_parameter_the_path_lacks_panics() {
        let _ = document(
            RouterBuilder::new().route(
                route(Method::GET, "/users")
                    .with(Operation::new().parameter(Parameter::path("post_id"))),
            ),
        );
    }

    #[test]
    fn other_parameters_are_optional_unless_marked_required() {
        let document = document(
            RouterBuilder::new().route(
                route(Method::GET, "/users").with(
                    Operation::new()
                        .parameter(Parameter::header("x-tenant").required(true))
                        .parameter(Parameter::query("page")),
                ),
            ),
        );

        assert_eq!(
            parameters(&document["paths"]["/users"]["get"]),
            [("x-tenant", "header", true), ("page", "query", false)]
        );
    }

    #[derive(JsonSchema)]
    #[allow(dead_code)]
    enum Order {
        Newest,
        Oldest,
    }

    #[derive(JsonSchema)]
    #[allow(dead_code)]
    struct PostsQuery {
        /// The text to search for.
        q: String,
        page: Option<u32>,
        order: Option<Order>,
    }

    #[test]
    fn the_fields_of_a_query_struct_become_query_parameters() {
        let document = document(
            RouterBuilder::new()
                .route(route(Method::GET, "/posts").with(Operation::new().query::<PostsQuery>())),
        );

        let operation = &document["paths"]["/posts"]["get"];
        let mut listed = parameters(operation);
        listed.sort_unstable();
        assert_eq!(
            listed,
            [
                ("order", "query", false),
                ("page", "query", false),
                ("q", "query", true)
            ]
        );

        let parameter = |name: &str| {
            operation["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .find(|parameter| parameter["name"] == name)
                .unwrap()
        };
        assert_eq!(parameter("q")["description"], "The text to search for.");
        // The description belongs to the parameter, not to its schema.
        assert!(parameter("q")["schema"].get("description").is_none());
        // An optional parameter is left out, never sent as `null`.
        assert_eq!(parameter("page")["schema"]["type"], "integer");
        let order = resolve(&document, &parameter("order")["schema"]);
        assert!(!order.to_string().contains("null"), "{order}");
    }

    #[test]
    fn a_described_query_parameter_takes_priority_over_a_field() {
        let document = document(
            RouterBuilder::new().route(
                route(Method::GET, "/posts").with(
                    Operation::new()
                        .query::<PostsQuery>()
                        .parameter(Parameter::query("page").description("Described.")),
                ),
            ),
        );

        let pages: Vec<_> = document["paths"]["/posts"]["get"]["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|parameter| parameter["name"] == "page")
            .collect();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0]["description"], "Described.");
    }

    #[test]
    #[should_panic(expected = "query parameters")]
    fn a_query_type_without_named_fields_panics() {
        let _ = document(
            RouterBuilder::new()
                .route(route(Method::GET, "/posts").with(Operation::new().query::<Vec<String>>())),
        );
    }

    // -- Bodies --

    #[test]
    fn request_and_response_bodies_refer_to_their_schemas() {
        let document = document(
            RouterBuilder::new().route(
                route(Method::POST, "/users").with(
                    Operation::new()
                        .request::<Json<CreateUser>>()
                        .response_with::<Json<User>>(StatusCode::CREATED, "The new user.")
                        .response(StatusCode::CONFLICT, "The name is taken."),
                ),
            ),
        );

        let operation = &document["paths"]["/users"]["post"];
        let request = &operation["requestBody"];
        assert_eq!(request["required"], true);
        let input = resolve(&document, &request["content"]["application/json"]["schema"]);
        assert_eq!(input["required"], json!(["name"]));

        let created = &operation["responses"]["201"];
        assert_eq!(created["description"], "The new user.");
        let output = resolve(&document, &created["content"]["application/json"]["schema"]);
        assert!(output["properties"].get("id").is_some());

        let conflict = &operation["responses"]["409"];
        assert_eq!(conflict["description"], "The name is taken.");
        assert!(conflict.get("content").is_none());
    }

    #[test]
    fn a_type_used_by_several_bodies_is_listed_once() {
        let document = document(
            RouterBuilder::new()
                .route(route(Method::GET, "/users/{id}").with(
                    Operation::new().response_with::<Json<User>>(StatusCode::OK, "The user."),
                ))
                .route(route(Method::GET, "/users").with(
                    Operation::new().response_with::<Json<Vec<User>>>(StatusCode::OK, "All users."),
                )),
        );

        let one = &document["paths"]["/users/{id}"]["get"]["responses"]["200"]["content"]["application/json"]
            ["schema"];
        let all = &document["paths"]["/users"]["get"]["responses"]["200"]["content"]["application/json"]
            ["schema"];
        assert!(one.get("$ref").is_some());
        assert_eq!(all["items"], *one);
        assert_eq!(
            document["components"]["schemas"].as_object().unwrap().len(),
            1
        );
    }

    #[test]
    fn an_optional_request_body_is_not_required() {
        let document = document(
            RouterBuilder::new().route(
                route(Method::POST, "/users")
                    .with(Operation::new().request::<Option<Json<CreateUser>>>()),
            ),
        );

        let request = &document["paths"]["/users"]["post"]["requestBody"];
        assert!(request["content"].get("application/json").is_some());
        assert_ne!(request.get("required"), Some(&json!(true)));
    }

    // -- Security --

    #[test]
    fn an_operation_lists_the_security_schemes_it_accepts() {
        let api = OpenApi::new("Test", "1.0.0")
            .security_scheme("token", SecurityScheme::bearer())
            .security_scheme("key", SecurityScheme::api_key_header("x-api-key"));
        let document = document_with(
            &api,
            RouterBuilder::new()
                .layer(layer("/api").with(Operation::new().security("token").security("key")))
                .route(route(Method::GET, "/api/users"))
                .route(route(Method::GET, "/api/health").with(Operation::new().no_security())),
        );

        assert_eq!(
            document["paths"]["/api/users"]["get"]["security"],
            json!([{ "token": [] }, { "key": [] }])
        );
        // An empty list documents the route as public. Leaving the field out
        // would not.
        assert_eq!(
            document["paths"]["/api/health"]["get"]["security"],
            json!([])
        );
        let schemes = document["components"]["securitySchemes"]
            .as_object()
            .unwrap();
        assert_eq!(schemes["token"]["scheme"], "bearer");
        assert_eq!(schemes["key"]["in"], "header");
        assert_eq!(schemes["key"]["name"], "x-api-key");
    }

    #[test]
    #[should_panic(expected = "tokne")]
    fn requiring_an_unregistered_security_scheme_panics() {
        let api = OpenApi::new("Test", "1.0.0").security_scheme("token", SecurityScheme::bearer());
        let _ = document_with(
            &api,
            RouterBuilder::new()
                .route(route(Method::GET, "/users").with(Operation::new().security("tokne"))),
        );
    }

    // -- Operation IDs --

    #[test]
    #[should_panic(expected = "list_users")]
    fn two_operations_sharing_an_id_panic() {
        let _ = document(
            RouterBuilder::new()
                .route(
                    route(Method::GET, "/users").with(Operation::new().operation_id("list_users")),
                )
                .route(
                    route(Method::GET, "/people").with(Operation::new().operation_id("list_users")),
                ),
        );
    }

    // -- The document as a whole --

    #[test]
    fn the_settings_describe_the_api() {
        let api = OpenApi::new("Acme API", "2.1.0")
            .description("Everything for tea.")
            .server("https://api.example.com")
            .tag("users", "Manage users.");
        let document = document_with(&api, RouterBuilder::new());

        assert!(document["openapi"].as_str().unwrap().starts_with("3.1."));
        assert_eq!(document["info"]["title"], "Acme API");
        assert_eq!(document["info"]["version"], "2.1.0");
        assert_eq!(document["info"]["description"], "Everything for tea.");
        assert_eq!(document["servers"][0]["url"], "https://api.example.com");
        assert_eq!(
            document["tags"],
            json!([{ "name": "users", "description": "Manage users." }])
        );
        // A document has to list its paths, even when there are none.
        assert_eq!(document["paths"], json!({}));
    }

    #[test]
    fn the_document_is_sent_as_json() {
        let router = RouterBuilder::new()
            .route(route(Method::GET, "/users").with(Operation::new()))
            .build();
        let document = OpenApi::new("Test", "1.0.0").generate(&router);

        let response = (&document).into_response(&Cx::new()).unwrap();

        assert_eq!(response.headers()["content-type"], "application/json");
        assert_eq!(
            serde_json::from_str::<Value>(&document.to_json()).unwrap(),
            serde_json::to_value(&document).unwrap()
        );
    }
}
