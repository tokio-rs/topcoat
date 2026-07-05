use topcoat::{
    Result,
    icon::{IconData, icon, iconify},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{attributes, svg::ViewBox, view},
};

const TRASH: IconData = IconData::unescaped_unchecked(
    ViewBox::new(0.0, 0.0, 24.0, 24.0),
    r#"<path fill="currentColor" d="M19,4H15.5L14.5,3H9.5L8.5,4H5V6H19V4M6,19A2,2 0 0,0 8,21H16A2,2 0 0,0 18,19V7H6V19Z"/>"#,
);

// The whole staged `demo` set as a module: `demo::TRASH`, `demo::BANNER`, ...
iconify::include!("demo");

// A single icon, through its alias chain, with a visibility.
iconify::include!(pub(crate) "demo:wastebasket");

// A hidden icon is skipped by globs but still works by name.
iconify::include!("demo:old-trash");

// Every icon inlined into a scope of its own.
mod all {
    topcoat::icon::iconify::include!(pub(super) "demo:*");
}

// The expression form composes with your own consts.
const AROUND_THE_CLOCK: IconData = iconify::iconify_icon!("demo:24-hours");

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[page("/")]
async fn home() -> Result {
    // Build an icon dynamically with `view!`:
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
                    icon(data: TRASH, size: 48)
                </p>
                <p>
                    "This one has an accessible label instead of aria-hidden: "
                    icon(data: TRASH, label: "Delete")
                </p>
                <p>
                    "And this one gets extra attributes forwarded to the svg element: "
                    icon(
                        data: target,
                        attrs: attributes! { style="color: rebeccapurple" }
                    )
                </p>
                <h2>"Iconify"</h2>
                <p>
                    "Icons from the staged demo set, included as a module: "
                    icon(data: demo::TRASH)
                    " "
                    icon(data: demo::TARGET)
                </p>
                <p>
                    "Named consts follow alias chains ("
                    icon(data: WASTEBASKET)
                    ") and reach hidden icons ("
                    icon(data: OLD_TRASH)
                    ")"
                </p>
                <p>
                    "Globs inline every visible icon into the scope: "
                    icon(data: all::_24_HOURS)
                    " "
                    icon(data: all::BIN)
                </p>
                <p>
                    "The expression form works in const contexts: "
                    icon(data: AROUND_THE_CLOCK)
                </p>
                <p>
                    "View boxes come from the set, so wide icons stay wide: "
                    icon(data: demo::BANNER, size: 24)
                </p>
            </body>
        </html>
    }
}
