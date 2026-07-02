use topcoat::{context::Cx, view::view};

fn r(v: topcoat::Result) -> String {
    v.unwrap().render(&Cx::empty())
}

#[tokio::test]
async fn if_true_branch_emits_its_body() {
    let signed_in = true;
    let html = r(view! {
        if signed_in {
            <a href="/account">"Account"</a>
        } else {
            <a href="/login">"Sign in"</a>
        }
    });

    assert_eq!(html, r#"<a href="/account">Account</a>"#);
}

#[tokio::test]
async fn if_false_branch_emits_else_body() {
    let signed_in = false;
    let html = r(view! {
        if signed_in {
            <a href="/account">"Account"</a>
        } else {
            <a href="/login">"Sign in"</a>
        }
    });

    assert_eq!(html, r#"<a href="/login">Sign in</a>"#);
}

#[tokio::test]
async fn if_without_else_emits_nothing_on_false() {
    let show = false;
    let html = r(view! {
        <div>
            if show {
                <p>"shown"</p>
            }
        </div>
    });

    assert_eq!(html, "<div></div>");
}

#[tokio::test]
async fn if_else_if_else_chain_selects_first_match() {
    let n = 1;
    let html = r(view! {
        if n == 0 {
            <p>"zero"</p>
        } else if n == 1 {
            <p>"one"</p>
        } else {
            <p>"many"</p>
        }
    });

    assert_eq!(html, "<p>one</p>");
}

#[tokio::test]
async fn if_in_attribute_list_adds_branch_attributes() {
    let current = true;
    let html = r(view! {
        <a
            href="/posts"
            if current {
                aria-current="page"
                class="active"
            }
        >
            "Posts"
        </a>
    });

    assert!(html.contains(r#"href="/posts""#));
    assert!(html.contains(r#"aria-current="page""#));
    assert!(html.contains(r#"class="active""#));
}

#[tokio::test]
async fn for_loop_renders_body_per_item() {
    let posts = ["alpha", "beta", "gamma"];
    let html = r(view! {
        <ul>
            for title in posts {
                <li>(title)</li>
            }
        </ul>
    });

    assert_eq!(html, "<ul><li>alpha</li><li>beta</li><li>gamma</li></ul>");
}

#[tokio::test]
async fn for_loop_in_attribute_list_emits_attributes_per_item() {
    let extras = [("data-a", "1"), ("data-b", "2")];
    let html = r(view! {
        <div
            for (name, value) in extras {
                (name)=(value)
            }
        >

        </div>
    });

    assert!(html.contains(r#"data-a="1""#));
    assert!(html.contains(r#"data-b="2""#));
}

#[tokio::test]
async fn for_loop_filtering_with_if_emits_subset() {
    let items = ["keep", "drop", "keep"];
    let html = r(view! {
        <ul>
            for item in items {
                if item == "keep" {
                    <li>(item)</li>
                }
            }
        </ul>
    });

    assert_eq!(html, "<ul><li>keep</li><li>keep</li></ul>");
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum Status {
    Draft,
    Published,
    Archived,
}

#[tokio::test]
async fn match_chooses_arm_body() {
    let html = r(view! {
        match Status::Published {
            Status::Draft => <span>"draft"</span>,
            Status::Published => <a href="/post">"open"</a>,
            Status::Archived => <span>"archived"</span>,
        }
    });

    assert_eq!(html, r#"<a href="/post">open</a>"#);
}

#[tokio::test]
async fn match_arm_with_block_emits_multiple_siblings() {
    let user = Some("ada");
    let html = r(view! {
        match user {
            Some(name) => {
                <h1>(name)</h1>
                <p>"signed in"</p>
            }
            None => <a href="/login">"sign in"</a>,
        }
    });

    assert_eq!(html, "<h1>ada</h1><p>signed in</p>");
}

#[tokio::test]
async fn match_in_attribute_list_emits_attribute_per_arm() {
    let status = Status::Draft;
    let html = r(view! {
        <article
            match status {
                Status::Draft => class="draft",
                Status::Published => class="published",
                Status::Archived => class="archived",
            }
        >

        </article>
    });

    assert_eq!(html, r#"<article class="draft"></article>"#);
}

#[tokio::test]
async fn let_binding_introduces_variable_for_following_nodes() {
    let html = r(view! {
        <article>
            let title = "  Hello  ".trim();

            <h1>(title)</h1>
            <p>(title)</p>
        </article>
    });

    assert_eq!(html, "<article><h1>Hello</h1><p>Hello</p></article>");
}

#[tokio::test]
async fn let_binding_in_attribute_list_is_in_scope_for_later_attributes() {
    let html = r(view! {
        <a let href = "/posts"; href=(href) data-href=(href)>"Posts"</a>
    });

    assert!(html.contains(r#"href="/posts""#));
    assert!(html.contains(r#"data-href="/posts""#));
}
