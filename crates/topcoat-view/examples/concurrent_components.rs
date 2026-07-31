use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    view::{component, view},
};

#[component]
async fn card(label: &'static str, delay: Duration) -> Result {
    println!("starting {label}");
    tokio::time::sleep(delay).await;
    println!("finished {label}");

    view! { <article>(label)</article> }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cx = Cx::default();
    let cx = &cx;
    let page = view! {
        cx =>
        <main>
            card(label: "first", delay: Duration::from_secs(1))
            <section>
                card(label: "second", delay: Duration::from_secs(1))
            </section>
        </main>
    }?;

    println!("{}", page.render(cx));
    Ok(())
}
