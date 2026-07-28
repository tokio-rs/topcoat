# Toasty todo

This example demonstrates how to use the Toasty ORM in a Topcoat application.

It shows how to:

- define a database model;
- initialize a SQLite database;
- store the database handle in application context;
- create, read, update, and delete todos;
- parse form data and dynamic path parameters;
- use Post/Redirect/Get after form submissions.

The example uses an in-memory SQLite database. Todos are lost when the server stops.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/toasty-todo/Cargo.toml
```

From inside the `examples` directory, run:

```sh
cargo run --manifest-path toasty-todo/Cargo.toml
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

## Test in the browser

Open the application.

The empty list should initially display:

```text
All done!
```

Enter a todo such as:

```text
Test Topcoat
```

Click **Add**.

The new todo should appear below the form.

Click its checkbox. The title should be displayed with a strikethrough.

Click the checkbox again. The todo should return to its incomplete state.

Click **delete**. The todo should disappear.

## Test with curl

Create a todo:

```sh
curl --include \
    --request POST \
    --header "Content-Type: application/x-www-form-urlencoded" \
    --data "title=Test%20Topcoat" \
    http://127.0.0.1:3000/todos
```

The response should redirect to:

```text
/
```

Request the page:

```sh
curl --silent http://127.0.0.1:3000/
```

The response should contain:

```text
Test Topcoat
```

The first todo created after starting the server normally has ID `1`.

Toggle it:

```sh
curl --include \
    --request POST \
    http://127.0.0.1:3000/todos/1/toggle
```

Request the page again:

```sh
curl --silent http://127.0.0.1:3000/
```

The todo title should now be rendered inside an `<s>` element.

Delete it:

```sh
curl --include \
    --request POST \
    http://127.0.0.1:3000/todos/1/delete
```

The page should return to:

```text
All done!
```

## Test database reset

Create one or more todos and stop the server with `Ctrl+C`.

Start the application again and open the root page.

The list should be empty because the application connects to:

```text
sqlite::memory:
```

To persist the database during local experimentation, change the connection URL to:

```text
sqlite:todos.db
```

## How it works

- `#[derive(toasty::Model)]` generates the Toasty model implementation.
- `#[key]` marks the primary key.
- `#[auto]` lets SQLite generate the todo ID.
- `Db::builder()` configures the database.
- `.models(...)` registers the application's models.
- `.connect("sqlite::memory:")` creates an in-memory SQLite database.
- `push_schema()` creates the required tables.
- `.app_context(db)` makes the database available to request handlers.
- `Todo::all()` loads the todo list.
- `toasty::create!` inserts a todo.
- `toasty::update!` updates its completed state.
- `Todo::delete_by_id` removes it.
- `path_param::<TodoId>(cx)` reads the todo ID from the route.
- `see_other("/")` redirects the browser after each modification.

Stop the server by pressing `Ctrl+C`.