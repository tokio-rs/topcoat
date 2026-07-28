use topcoat::{
    Result,
    icon::{IconData, icon, iconify},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{svg::ViewBox, view},
};

// Define an icon manually using an SVG view box and body.
//
// `currentColor` makes the icon inherit the surrounding text color.
const TRASH: IconData = IconData::unescaped_unchecked(
    ViewBox::new(0.0, 0.0, 24.0, 24.0),
    r#"<path fill="currentColor" d="M19,4H15.5L14.5,3H9.5L8.5,4H5V6H19V4M6,19A2,2 0 0,0 8,21H16A2,2 0 0,0 18,19V7H6V19Z"/>"#,
);

// Include the Feather icon set staged by build.rs.
//
// The generated module contains IconData constants such as
// feather::TARGET and feather::FEATHER.
iconify::include!("feather");

#[tokio::main]
async fn main() {
    // Discover the page and start the HTTP server.
    // By default, the application is available at http://127.0.0.1:3000.
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[page("/")]
async fn home() -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Icons"</title>
                topcoat::dev::script()
            </head>

            <body>
                <p>
                    "An icon renders as an inline SVG that is 1em square by default, "
                    "so it matches the surrounding font size: "
                    icon(data: TRASH)
                </p>

                <p style="color: crimson">
                    "It also inherits the text color through currentColor: "
                    icon(data: TRASH)
                </p>

                <p>
                    "Pass a size to fix its dimensions, and a label to expose it to "
                    "assistive technology instead of hiding it: "
                    icon(data: TRASH, size: 48, label: "Delete")
                </p>

                <p>
                    "You can also include any icon of the Iconify collection, "
                    "downloaded and staged by the build script: "
                    icon(data: feather::TARGET)
                    " "
                    icon(data: feather::FEATHER)
                </p>
            </body>
        </html>
    }
}
