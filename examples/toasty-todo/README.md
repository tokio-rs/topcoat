# Toasty todo

A todo list backed by the Toasty ORM: a model, an in-memory SQLite database in app context, and create, toggle, and delete routes that redirect after each submission.

Run it with:

```sh
cargo topcoat dev -p toasty-todo
```

The database lives in memory, so the todos are gone when the server stops.
