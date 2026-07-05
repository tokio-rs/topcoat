use topcoat::{
    Result,
    icon::{IconData, icon},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{attributes, svg::ViewBox, view},
};

// The const way: store the raw SVG body verbatim in a `const`. This is the
// shape the iconify macros expand to, but nothing stops you from writing it
// by hand.
const TRASH: IconData = IconData::unescaped_unchecked(
    ViewBox::new(0.0, 0.0, 24.0, 24.0),
    r#"<path fill="currentColor" d="M19,4H15.5L14.5,3H9.5L8.5,4H5V6H19V4M6,19A2,2 0 0,0 8,21H16A2,2 0 0,0 18,19V7H6V19Z"/>"#,
);

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[page("/")]
async fn home() -> Result {
    // The dynamic way: build the body with `view!`, self-closing tags and all.
    let target = IconData::new(
        ViewBox::new(0.0, 0.0, 24.0, 24.0),
        view! {
            <circle
                cx="12"
                cy="12"
                r="10"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
            />
            <circle
                cx="12"
                cy="12"
                r="6"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
            />
            <circle cx="12" cy="12" r="2" fill="currentColor" />
        }?,
    );

    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Icons"</title>
                topcoat::dev::script()
            </head>
            <body>
                <h1>
                    icon(data: target.clone())
                    " Icons"
                </h1>
                <p>
                    "Icons are 1em squares by default, so they follow the font size of the surrounding text: "
                    icon(data: TRASH)
                </p>
                <p style="color: crimson">
                    "They also inherit the text color through currentColor: "
                    icon(data: TRASH)
                </p>
                <p>
                    "This one is a fixed 48 pixels: "
                    icon(
                        data: TRASH,
                        size: 48
                    )
                </p>
                <p>
                    "This one has an accessible label instead of aria-hidden: "
                    icon(
                        data: TRASH,
                        label: "Delete".to_string()
                    )
                </p>
                <p>
                    "And this one gets extra attributes forwarded to the svg element: "
                    icon(
                        data: target,
                        attrs: attributes! { style="color: rebeccapurple" }
                    )
                </p>
            </body>
        </html>
    }
}
