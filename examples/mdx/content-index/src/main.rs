use std::{collections::HashMap, sync::LazyLock};

use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    mdx::{MdxFrontmatterFormat, MdxIndexEntry, mdx_pages},
    router::{
        Router, RouterBuilderDiscoverExt, layout, page, path_param_segment, response::Response,
        route,
    },
    view::{component, view},
};

// --- Content -----------------------------------------------------------------

// Scanning a directory does two things: it registers a route per file, and it
// reads each file's frontmatter into a compile-time index. Nothing here touches
// the filesystem at runtime.
mdx_pages!("posts", prefix = "/posts");

/// The index in newest-first order. `date` is a string, and the frontmatter
/// uses ISO-8601, so a plain reverse sort is chronological.
fn posts_by_date() -> Vec<&'static MdxIndexEntry> {
    let mut posts: Vec<_> = mdx_index_posts().iter().collect();
    posts.sort_by(|a, b| b.date.unwrap_or_default().cmp(a.date.unwrap_or_default()));
    posts
}

// --- Custom frontmatter ------------------------------------------------------

// `MdxIndexEntry` names four frontmatter fields. Everything else a post
// declares arrives as a raw string, so a type of your own describes the rest.
#[derive(Deserialize)]
struct PostMeta {
    subtitle: Option<String>,
    author: Option<String>,
}

/// The custom fields of every post, parsed once and keyed by slug.
///
/// Deserializing is a runtime cost, so a listing that renders on every request
/// should not repeat it. Posts whose frontmatter does not match are left out
/// rather than bringing the page down.
static POST_META: LazyLock<HashMap<&'static str, PostMeta>> = LazyLock::new(|| {
    mdx_index_posts()
        .iter()
        .filter_map(|post| {
            let meta = match post.frontmatter_format {
                // The delimiters are stripped during parsing, so the syntax
                // has to be read from the entry rather than from the string.
                MdxFrontmatterFormat::Yaml => serde_saphyr::from_str(post.frontmatter_raw).ok()?,
                MdxFrontmatterFormat::Toml => toml::from_str(post.frontmatter_raw).ok()?,
                MdxFrontmatterFormat::None => return None,
            };
            Some((post.slug, meta))
        })
        .collect()
});

/// Every tag used across the scanned files, deduplicated and sorted.
fn all_tags() -> Vec<&'static str> {
    let mut tags: Vec<&'static str> = mdx_index_posts()
        .iter()
        .flat_map(|post| post.tags.iter().copied())
        .collect();
    tags.sort_unstable();
    tags.dedup();
    tags
}

// --- Views -------------------------------------------------------------------

#[component]
async fn post_card(post: &'static MdxIndexEntry) -> Result {
    let meta = POST_META.get(post.slug);
    // Words per minute is a presentation choice, which is why the index
    // reports a word count rather than a formatted string.
    let minutes = post.word_count.div_ceil(200).max(1);

    view! {
        <li class="border-b py-4">
            <a href=(post.path) class="text-lg font-medium">
                (post.title.unwrap_or(post.slug))
            </a>
            if let Some(subtitle) = meta
                .and_then(|meta| meta.subtitle.as_deref()) {
                <p class="text-gray-600">(subtitle)</p>
            }
            <p class="text-sm text-gray-500">
                (post.date.unwrap_or("undated"))
                if let Some(author) = meta
                    .and_then(|meta| meta.author.as_deref()) {
                    " by "
                    (author)
                }
                " - "
                (minutes)
                " min read"
            </p>
            if let Some(excerpt) = post.excerpt {
                <p class="mt-1">(excerpt)</p>
            }
            <p class="mt-1 text-sm">
                for tag in post.tags {
                    <a href=(format!("/tags/{tag}")) class="mr-2">
                        "#"
                        (tag)
                    </a>
                }
            </p>
        </li>
    }
}

// --- Pages -------------------------------------------------------------------

#[page("/")]
async fn index() -> Result {
    let posts = posts_by_date();
    view! {
        <h1 class="text-2xl font-semibold">"All posts"</h1>
        <ul>
            for post in posts {
                post_card(post: post)
            }
        </ul>
    }
}

#[page("/tags/{tag}")]
async fn tag_page(cx: &Cx) -> Result {
    let tag = path_param_segment(cx, "tag");
    let posts: Vec<_> = mdx_index_posts()
        .iter()
        .filter(|post| post.tags.contains(&tag))
        .collect();

    view! {
        <h1 class="text-2xl font-semibold">
            "Tagged "
            (tag)
        </h1>
        if posts.is_empty() {
            <p>"No posts carry this tag."</p>
        } else {
            <ul>
                for post in posts {
                    post_card(post: post)
                }
            </ul>
        }
    }
}

// The index is a plain slice, so it feeds non-HTML responses just as well.
#[route(GET "/sitemap.xml")]
async fn sitemap() -> Result<Response> {
    use std::fmt::Write;

    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);
    for post in mdx_index_posts() {
        write!(xml, "<url><loc>{}</loc></url>", post.path)
            .expect("writing to a String cannot fail");
    }
    xml.push_str("</urlset>");

    Ok(Response::builder()
        .header("content-type", "application/xml")
        .body(xml.into())
        .expect("sitemap response is well-formed"))
}

// --- Layout ------------------------------------------------------------------

#[layout("/")]
async fn root_layout(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"MDX Content Index"</title>
                topcoat::dev::script()
            </head>
            <body>
                <nav class="border-b px-6 py-3">
                    <a href="/" class="font-semibold">"Posts"</a>
                    " | "
                    for tag in all_tags() {
                        <a href=(format!("/tags/{tag}")) class="mr-2">
                            "#"
                            (tag)
                        </a>
                    }
                    " | "
                    <a href="/sitemap.xml">"Sitemap"</a>
                </nav>
                <main class="mx-auto max-w-2xl px-6 py-8">(slot?)</main>
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
