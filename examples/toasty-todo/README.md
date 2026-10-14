# Toasty todo

A todo list backed by the Toasty ORM. Form submissions update the database and redirect to the list.

Clone the Topcoat repository:

```sh
git clone https://github.com/tokio-rs/topcoat.git
cd topcoat
```

Then run it with:

```sh
cargo topcoat dev -p toasty-todo
```

The database lives in memory, so the todos are gone when the server stops.
