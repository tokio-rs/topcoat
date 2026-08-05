use serde::Deserialize;
use topcoat::{
    Result,
    mdx::{MdxIndexEntry, mdx_pages},
    router::{Router, RouterBuilderDiscoverExt, layout, page},
    view::{View, component, view},
};

// --- Metadata ----------------------------------------------------------------

// The shape every post in `posts/` is read into. Four of these fields also have
// named slots on `MdxIndexEntry`; the rest would be dropped without a type to
// hold them.
//
// Fields that not every post declares are `Option`, which is what lets one type
// serve a directory whose pages differ.
#[derive(Deserialize)]
struct BlogPostMeta {
    title: String,
    subtitle: Option<String>,
    author: String,
    #[serde(rename = "publishDate")]
    publish_date: String,
    #[serde(rename = "lastModifiedDate")]
    last_modified_date: String,
    excerpt: String,
    tags: Vec<String>,
    keywords: Option<Vec<String>>,
}

// --- Content -----------------------------------------------------------------

// `frontmatter = BlogPostMeta` makes the macro deserialize each page itself.
// It reads the syntax each page used and calls the matching deserializer, so
// the YAML and TOML posts here arrive as the same type without this code
// choosing between them.
//
// This argument needs the `mdx-frontmatter` feature. Rendering MDX resolves
// frontmatter while the macro expands and keeps none of it; reading it into a
// type of your own happens in the running program instead.
mdx_pages!(
    "posts",
    prefix = "/blog",
    frontmatter = BlogPostMeta,
    wrapper = post_layout,
);

/// Posts that carry frontmatter, most recently modified first.
///
/// A page without frontmatter has no date to sort by, so it is left out rather
/// than sorted arbitrarily.
fn posts_by_modified() -> Vec<(&'static MdxIndexEntry<BlogPostMeta>, &'static BlogPostMeta)> {
    let mut posts: Vec<_> = mdx_index_posts()
        .iter()
        .filter_map(|post| post.meta().map(|meta| (post, meta)))
        .collect();
    posts.sort_by(|(_, a), (_, b)| b.last_modified_date.cmp(&a.last_modified_date));
    posts
}

// --- Views -------------------------------------------------------------------

// The wrapper every page in `posts/` renders through. `meta` arrives as a prop
// because the macro was given a frontmatter type, and is an `Option` because
// `no-frontmatter.mdx` has nothing to pass.
#[component]
async fn post_layout(#[default] child: View, meta: Option<&'static BlogPostMeta>) -> Result {
    view! {
        <article class="prose mx-auto max-w-2xl px-6 py-8">
            if let Some(meta) = meta {
                <header class="mb-8 border-b pb-4">
                    <h1 class="text-3xl font-semibold">(&meta.title)</h1>
                    if let Some(subtitle) = &meta.subtitle {
                        <p class="text-xl text-gray-600">(subtitle)</p>
                    }
                    <p class="mt-2 text-sm text-gray-500">
                        "By "
                        (&meta.author)
                        " - published "
                        (&meta.publish_date)
                        " - updated "
                        (&meta.last_modified_date)
                    </p>
                    <p class="mt-1 text-sm text-gray-500">
                        for tag in &meta.tags {
                            <a href=(format!("/tags/{tag}")) class="mr-2">
                                "#"
                                (tag)
                            </a>
                        }
                    </p>
                </header>
            }
            (child)
        </article>
    }
}

#[component]
async fn post_card(
    post: &'static MdxIndexEntry<BlogPostMeta>,
    meta: &'static BlogPostMeta,
) -> Result {
    // The index counts words so that an estimate is possible at all: the page
    // body is compiled away and never exists as text at runtime. Words per
    // minute stays a choice for this page to make.
    let minutes = post.word_count.div_ceil(200).max(1);

    view! {
        <li class="border-b py-4">
            <a href=(post.path) class="text-lg font-medium">(&meta.title)</a>
            if let Some(subtitle) = &meta.subtitle {
                <p class="text-gray-600">(subtitle)</p>
            }
            <p class="text-sm text-gray-500">
                (&meta.publish_date)
                " - "
                (minutes)
                " min read"
            </p>
            <p class="mt-1">(&meta.excerpt)</p>
            if let Some(keywords) = &meta.keywords {
                <p class="mt-1 text-sm text-gray-500">
                    "Keywords: "
                    (keywords.join(", "))
                </p>
            }
        </li>
    }
}

// --- Pages -------------------------------------------------------------------

#[page("/")]
async fn index() -> Result {
    view! {
        <h1 class="text-2xl font-semibold">"Posts"</h1>
        <ul>
            for (post, meta) in posts_by_modified() {
                post_card(post: post, meta: meta)
            }
        </ul>
    }
}

// --- Layout ------------------------------------------------------------------

#[layout("/")]
async fn root_layout(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"MDX Frontmatter"</title>
                topcoat::dev::script()
            </head>
            <body>
                <nav class="border-b px-6 py-3">
                    <a href="/" class="font-semibold">"Posts"</a>
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

#[cfg(test)]
mod tests {
    use super::{BlogPostMeta, mdx_index_posts, posts_by_modified};

    fn meta(slug: &str) -> Option<&'static BlogPostMeta> {
        mdx_index_posts()
            .iter()
            .find(|post| post.slug == slug)
            .unwrap_or_else(|| panic!("{slug} should be scanned"))
            .meta()
    }

    #[test]
    fn yaml_post_metadata_parses() {
        let meta = meta("typed-metadata").expect("the YAML post has frontmatter");
        assert_eq!(meta.title, "Typed Metadata");
        assert_eq!(
            meta.subtitle.as_deref(),
            Some("One struct describes what every post carries")
        );
        assert_eq!(meta.author, "Topcoat");
        assert_eq!(meta.publish_date, "2026-01-20");
        assert_eq!(meta.last_modified_date, "2026-02-14");
        assert_eq!(meta.tags, ["mdx", "frontmatter"]);
        assert_eq!(meta.keywords.as_ref().map(Vec::len), Some(2));
    }

    #[test]
    fn toml_post_metadata_parses() {
        let meta = meta("toml-frontmatter").expect("the TOML post has frontmatter");
        assert_eq!(meta.title, "TOML Frontmatter");
        assert_eq!(
            meta.subtitle.as_deref(),
            Some("The same fields, a different syntax")
        );
        assert_eq!(meta.author, "Topcoat");
        assert_eq!(meta.publish_date, "2026-02-02");
        assert_eq!(meta.tags, ["mdx", "toml"]);
    }

    // Both syntaxes reach the same type with every field populated, so neither
    // is quietly losing fields the other keeps.
    #[test]
    fn both_formats_yield_the_same_shape() {
        for slug in ["typed-metadata", "toml-frontmatter"] {
            let meta = meta(slug).unwrap_or_else(|| panic!("{slug} has frontmatter"));
            assert!(!meta.title.is_empty(), "{slug} title");
            assert!(meta.subtitle.is_some(), "{slug} subtitle");
            assert!(!meta.excerpt.is_empty(), "{slug} excerpt");
            assert!(!meta.tags.is_empty(), "{slug} tags");
            assert!(meta.keywords.is_some(), "{slug} keywords");
        }
    }

    // A frontmatter type does not oblige every page to declare every field.
    #[test]
    fn partial_metadata_leaves_optional_fields_empty() {
        let meta = meta("partial-metadata").expect("the partial post has frontmatter");
        assert!(meta.subtitle.is_none());
        assert!(meta.keywords.is_none());
        assert_eq!(meta.title, "Partial Metadata");
    }

    #[test]
    fn post_without_frontmatter_has_no_meta() {
        assert!(meta("no-frontmatter").is_none());
        assert!(
            !posts_by_modified()
                .iter()
                .any(|(post, _)| post.slug == "no-frontmatter"),
            "a post with no date should not appear in a listing sorted by date"
        );
    }

    #[test]
    fn listing_is_newest_first() {
        let dates: Vec<&str> = posts_by_modified()
            .iter()
            .map(|(_, meta)| meta.last_modified_date.as_str())
            .collect();
        let mut sorted = dates.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(dates, sorted);
    }

    // A post that keeps its own directory is reached at the directory, not at
    // a route ending in a repeated segment.
    #[test]
    fn index_file_serves_its_directory() {
        let entry = mdx_index_posts()
            .iter()
            .find(|post| post.slug == "release-notes")
            .expect("the index file is indexed under its directory, not as `index`");
        assert_eq!(entry.path, "/blog/release-notes");
    }

    #[test]
    fn no_route_ends_in_index() {
        for post in mdx_index_posts() {
            assert!(!post.path.ends_with("/index"), "{}", post.path);
            assert_ne!(post.slug, "index");
        }
    }

    // Parsing happens once, not on every read.
    #[test]
    fn metadata_parses_once() {
        let first = meta("typed-metadata").expect("frontmatter is present");
        let second = meta("typed-metadata").expect("frontmatter is present");
        assert!(std::ptr::eq(first, second));
    }
}
