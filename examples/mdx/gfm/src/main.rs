use topcoat::{
    Result,
    mdx::compile_mdx,
    router::{Router, RouterBuilderDiscoverExt, layout, page},
    view::{View, component, view},
};

// --- Code block component -----------------------------------------------------

// A fenced code block's meta string arrives as `data-*` props. Overriding
// `pre` is how a highlighting component gets at them: the language decides the
// highlighter, the line ranges and emphasis terms decide what to mark up.
#[component]
pub async fn highlighted_code(
    #[default] data_lang: &'static str,
    #[default] data_lines: &'static str,
    #[default] data_title: &'static str,
    #[default] data_emphasis: &'static str,
    #[default] child: View,
) -> Result {
    view! {
        <figure class="my-4 overflow-hidden rounded border">
            <figcaption class="flex justify-between border-b px-3 py-1 text-sm">
                <span>
                    if data_title.is_empty() {
                        "snippet"
                    } else {
                        (data_title)
                    }
                </span>
                <span class="text-gray-500">
                    if data_lang.is_empty() {
                        "text"
                    } else {
                        (data_lang)
                    }
                </span>
            </figcaption>
            <pre class="overflow-x-auto p-3">(child)</pre>
            if !data_lines.is_empty() || !data_emphasis.is_empty() {
                <figcaption class="border-t px-3 py-1 text-xs text-gray-500">
                    if !data_lines.is_empty() {
                        "highlight lines "
                        (data_lines)
                        " "
                    }
                    if !data_emphasis.is_empty() {
                        "emphasize "
                        (data_emphasis)
                    }
                </figcaption>
            }
        </figure>
    }
}

// --- Pages -------------------------------------------------------------------

#[page("/")]
async fn index() -> Result {
    compile_mdx!(
        {},
        overrides = { "pre" => highlighted_code },
        "pages/index.mdx"
    )
}

// --- Layout ------------------------------------------------------------------

#[layout]
async fn root_layout(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"MDX GFM"</title>
                topcoat::dev::script()
            </head>
            <body><main class="mx-auto max-w-2xl px-6 py-8">(slot?)</main></body>
        </html>
    }
}

// --- Server ------------------------------------------------------------------

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}
