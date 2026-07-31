use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    view::{component, view},
};

struct Post {
    title: &'static str,
    delay: Duration,
}

#[component]
async fn post_card(post: Post) -> Result {
    tokio::time::sleep(post.delay).await;
    println!("finished {}", post.title);

    view! { <article>(post.title)</article> }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let posts = vec![
        Post {
            title: "first",
            delay: Duration::from_millis(100),
        },
        Post {
            title: "second",
            delay: Duration::from_millis(50),
        },
    ];
    let cx = Cx::default();
    let cx = &cx;
    let page = view! {
        cx =>
        <main>
            for concurrent post in posts {
                post_card(post: post)
            }
        </main>
    }?;

    // "second" finishes first, but the generated HTML keeps iterator order.
    println!("{}", page.render(cx));
    Ok(())
}
