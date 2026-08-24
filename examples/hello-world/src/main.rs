use std::time::Duration;

use topcoat::{
    Result,
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{View, ViewHandle, component, emit, live, view},
};

#[tokio::main]
async fn main() {
    // `discover` picks up every page, layout, and route declared in the crate.
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

/// Applies streamed swaps: replaces the content between a live position's
/// marker comments with the template the swap arrived in.
// TODO: serve this from the framework instead of pasting it into the page.
const SWAP_SCRIPT: &str = r#"<script>
window.topcoat = {
    swap(id) {
        const script = document.currentScript;
        const template = script.previousElementSibling;
        let open = null;
        let close = null;
        const walker = document.createTreeWalker(document.documentElement, NodeFilter.SHOW_COMMENT);
        while (walker.nextNode()) {
            const comment = walker.currentNode;
            if (comment.data === `tc:${id}`) open = comment;
            else if (comment.data === `/tc:${id}`) close = comment;
        }
        if (open && close) {
            while (open.nextSibling && open.nextSibling !== close) open.nextSibling.remove();
            close.parentNode.insertBefore(template.content, close);
        }
        template.remove();
        script.remove();
    },
};
</script>"#;

#[page("/")]
async fn home() -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Hello world"</title>
                // Reloads the browser when the dev server rebuilds the app.
                // topcoat::dev::script() // TODO
                (ViewHandle::unescaped_unchecked(SWAP_SCRIPT))
            </head>
            <body>
                (live! {
                    for count in 0..100 {
                        emit! { "Loading... " (count) "%" }?;
                        tokio::time::sleep(Duration::from_millis(20)).await;
                    }
                    emit! { "Hello world!" }
                })
            </body>
        </html>
    })
}
