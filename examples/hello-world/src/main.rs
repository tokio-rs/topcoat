use topcoat::{component, view, view::View};

#[component]
async fn button<'a>(id: &'a str, child: View) -> View {
    view! {
        <button id=(id) class="button">(child)</button>
    }
}

#[tokio::main]
async fn main() {
    let content = view! {
        <html>
            <head>
                <title>"hello world"</title>
            </head>
            <body id="test">
                [button id="5"]
                    "click me"
                [/button]
            </body>
        </html>
    };

    println!("{}", content);
}
