use topcoat::{
    Result,
    mdx::compile_mdx,
    router::{Router, RouterBuilderDiscoverExt, layout, page},
    view::{View, component, view},
};

// --- Override components -----------------------------------------------------
//
// An override component stands in for a plain HTML element. The element's
// attributes arrive as named props, so a component must declare every
// attribute the walker can emit for that element. Attributes are only passed
// when present in the source, so anything optional needs `#[default]`.

// Every markdown link renders through this. Links always carry `href`; `title`
// is only present for `[text](url "Title")`.
#[component]
pub async fn branded_link(
    href: &'static str,
    #[default] title: &'static str,
    #[default] child: View,
) -> Result {
    view! {
        <a class="text-blue-600 underline" href=(href) title=(title)>
            (child)
            <span class="text-xs text-blue-400">" ->"</span>
        </a>
    }
}

// Headings carry the generated anchor id, so an override can render a
// permalink next to the text.
#[component]
pub async fn anchored_heading(id: &'static str, #[default] child: View) -> Result {
    view! {
        <h2 id=(id) class="group text-2xl font-semibold">
            (child)
            <a href=(format!("#{id}")) class="ml-2 opacity-0 group-hover:opacity-100">
                "#"
            </a>
        </h2>
    }
}

// A fenced code block passes its meta string as `data-*` attributes. All of
// them are optional: a bare fence with no language emits none of them.
#[component]
pub async fn code_block(
    #[default] data_lang: &'static str,
    #[default] data_title: &'static str,
    #[default] child: View,
) -> Result {
    view! {
        <figure class="my-4 rounded border">
            if !data_title.is_empty() {
                <figcaption class="border-b px-3 py-1 text-sm">(data_title)</figcaption>
            }
            <pre class="overflow-x-auto p-3" data-lang=(data_lang)>(child)</pre>
        </figure>
    }
}

// --- Wrapper -----------------------------------------------------------------
//
// A wrapper receives the whole compiled document as its `child` prop, so the
// content does not have to know how it is framed.

#[component]
pub async fn prose(#[default] child: View) -> Result {
    view! { <article class="mx-auto max-w-2xl">(child)</article> }
}

// --- Pages -------------------------------------------------------------------

#[page("/")]
async fn index() -> Result {
    // Overrides apply to elements the author never writes as components: the
    // markdown stays plain, and the framework routes it through Rust.
    compile_mdx!(
        {},
        overrides = {
            "a" => branded_link,
            "h2" => anchored_heading,
            "pre" => code_block,
        },
        wrapper = prose,
        "pages/index.mdx"
    )
}

#[page("/plain")]
async fn plain() -> Result {
    // The same file with no overrides, for comparison.
    compile_mdx!("pages/index.mdx")
}

// --- Layout ------------------------------------------------------------------

#[layout]
async fn root_layout(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"MDX Overrides"</title>
                topcoat::dev::script()
            </head>
            <body>
                <nav class="border-b px-6 py-3">
                    <a href="/" class="font-semibold">"Overridden"</a>
                    " | "
                    <a href="/plain">"Plain"</a>
                </nav>
                <main class="px-6 py-8">(slot?)</main>
            </body>
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
