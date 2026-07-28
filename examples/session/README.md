# Session

This example demonstrates how to create login and logout flows with Topcoat sessions.

It shows how to:

- enable cookies and sessions on the router;
- create a session after receiving a login form;
- associate a session token with a user;
- read the current user from an incoming request;
- stop and remove a session during logout.

The example uses an in-memory database and does not verify passwords. It is intended only to demonstrate the session workflow.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/session/Cargo.toml
```

From inside the `examples` directory, run:

```sh
cargo run --manifest-path session/Cargo.toml
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

## Test in the browser

Open:

```text
http://127.0.0.1:3000
```

The page should initially display:

```text
currently not logged in
```

Enter a username and click **log in**.

The page should then display:

```text
currently logged in as: <username>
```

Click **log out**. The page should return to:

```text
currently not logged in
```

## Test with curl

Create a cookie jar and check the initial page:

```sh
curl --silent \
    --cookie-jar /tmp/topcoat-session.cookies \
    http://127.0.0.1:3000/
```

The response should contain:

```text
currently not logged in
```

Log in and store the session cookie:

```sh
curl --include \
    --request POST \
    --header "Content-Type: application/x-www-form-urlencoded" \
    --data "name=Francesco" \
    --cookie-jar /tmp/topcoat-session.cookies \
    http://127.0.0.1:3000/login
```

The response should redirect to `/`.

Request the page using the stored cookie:

```sh
curl --silent \
    --cookie /tmp/topcoat-session.cookies \
    http://127.0.0.1:3000/
```

The response should contain:

```text
currently logged in as: Francesco
```

Log out and update the cookie jar:

```sh
curl --include \
    --request POST \
    --cookie /tmp/topcoat-session.cookies \
    --cookie-jar /tmp/topcoat-session.cookies \
    http://127.0.0.1:3000/logout
```

Request the page again:

```sh
curl --silent \
    --cookie /tmp/topcoat-session.cookies \
    http://127.0.0.1:3000/
```

The response should contain:

```text
currently not logged in
```

## How it works

- `.cookies()` enables cookie support.
- `.sessions(...)` configures session management.
- `.app_context(Database::default())` registers the demo database.
- `session::start(cx)` creates a new session and cookie.
- The session token hash is associated with a user in the database.
- `session::token_hash(cx)` reads the active session token.
- `current_user` retrieves the user associated with that token.
- `session::stop(cx)` ends the session.
- Login and logout return `303 See Other` responses that redirect to `/`.

The database is stored only in memory. All users and sessions are lost when the server restarts.

Stop the server by pressing `Ctrl+C`.