Builds a streaming shell with inline deferred components.

Write the deferred component after `defer`. The block that follows it is sent as its placeholder:

```rust
# use topcoat::{Result, context::Cx, shell_view::{ShellView, shell_view}, view::{component, view}};
# #[component]
# async fn newsfeed() -> Result { view! { <p>"News"</p> } }
# async fn example(cx: &Cx) -> Result<ShellView> {
shell_view! {
    cx =>
    <main>
        defer newsfeed() {
            <p aria-busy="true">"Loading news..."</p>
        }
    </main>
}
# }
```

Each inline deferred component runs concurrently and replaces its placeholder when it finishes.
