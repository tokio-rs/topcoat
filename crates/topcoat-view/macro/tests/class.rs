use topcoat::{
    context::Cx,
    view::{Class, class, view},
};

fn r(v: topcoat::Result) -> String {
    v.unwrap().render(&Cx::default())
}

#[tokio::test]
async fn literal_entries_render_space_separated() {
    topcoat::view::scope(async {
        let cx = &Cx::default();
        let html = r(view! {
            cx =>
            <button class=(class!("btn", "btn-lg"))>"go"</button>
        });
        assert_eq!(html, r#"<button class="btn btn-lg">go</button>"#);
    })
    .await;
}

#[tokio::test]
async fn true_condition_includes_the_entry() {
    topcoat::view::scope(async {
        let is_active = true;
        let cx = &Cx::default();
        let html = r(view! { cx => <p class=(class!("btn", "active" if is_active))></p> });
        assert_eq!(html, r#"<p class="btn active"></p>"#);
    })
    .await;
}

#[tokio::test]
async fn false_condition_skips_the_entry_and_separator() {
    topcoat::view::scope(async {
        let is_active = false;
        let cx = &Cx::default();
        let html = r(view! { cx => <p class=(class!("btn", "active" if is_active))></p> });
        assert_eq!(html, r#"<p class="btn"></p>"#);
    })
    .await;
}

#[tokio::test]
async fn else_branch_renders_the_alternative() {
    topcoat::view::scope(async {
        let enabled = false;
        let cx = &Cx::default();
        let html = r(view! { cx => <p class=(class!("on" if enabled else "off"))></p> });
        assert_eq!(html, r#"<p class="off"></p>"#);
    })
    .await;
}

#[tokio::test]
async fn else_branch_with_expression_renders_the_taken_side() {
    topcoat::view::scope(async {
        let enabled = true;
        let fallback = String::from("fallback");
        let cx = &Cx::default();
        let html = r(view! {
            cx =>
            <p class=(class!("on" if enabled else fallback))></p>
        });
        assert_eq!(html, r#"<p class="on"></p>"#);
    })
    .await;
}

#[tokio::test]
async fn none_option_is_omitted_without_separator() {
    topcoat::view::scope(async {
        let variant: Option<&str> = None;
        let cx = &Cx::default();
        let html = r(view! { cx => <p class=(class!("btn", variant, "rounded"))></p> });
        assert_eq!(html, r#"<p class="btn rounded"></p>"#);
    })
    .await;
}

#[tokio::test]
async fn all_entries_absent_omits_the_attribute() {
    topcoat::view::scope(async {
        let variant: Option<&str> = None;
        let cx = &Cx::default();
        let html = r(view! { cx => <p class=(class!(variant, "active" if false))></p> });
        assert_eq!(html, "<p></p>");
    })
    .await;
}

#[tokio::test]
async fn empty_class_omits_the_attribute() {
    topcoat::view::scope(async {
        let cx = &Cx::default();
        let html = r(view! { cx => <p class=(class!())></p> });
        assert_eq!(html, "<p></p>");
    })
    .await;
}

#[tokio::test]
async fn dynamic_entries_are_escaped() {
    topcoat::view::scope(async {
        let value = String::from("a\"b");
        let cx = &Cx::default();
        let html = r(view! { cx => <p class=(class!(value))></p> });
        assert_eq!(html, r#"<p class="a&quot;b"></p>"#);
    })
    .await;
}

#[tokio::test]
async fn vec_entries_render_with_separators() {
    topcoat::view::scope(async {
        let sizes = vec!["px-4".to_owned(), "py-2".to_owned()];
        let cx = &Cx::default();
        let html = r(view! { cx => <p class=(class!("btn", sizes))></p> });
        assert_eq!(html, r#"<p class="btn px-4 py-2"></p>"#);
    })
    .await;
}

#[tokio::test]
async fn nested_class_is_spliced_with_separators() {
    topcoat::view::scope(async {
        let base: Class<_> = class!("btn", "btn-lg");
        let cx = &Cx::default();
        let html = r(view! { cx => <p class=(class!("card", base))></p> });
        assert_eq!(html, r#"<p class="card btn btn-lg"></p>"#);
    })
    .await;
}

#[tokio::test]
async fn trailing_comma_is_allowed() {
    topcoat::view::scope(async {
        let cx = &Cx::default();
        let html = r(view! { cx => <p class=(class!("a", "b"))></p> });
        assert_eq!(html, r#"<p class="a b"></p>"#);
    })
    .await;
}

#[tokio::test]
async fn more_than_twelve_entries_flatten() {
    topcoat::view::scope(async {
        let cx = &Cx::default();
        let e = "e";
        let html = r(view! {
            cx =>
            <p
                class=(class!(
                    (e),
                    (e),
                    (e),
                    (e),
                    (e),
                    (e),
                    (e),
                    (e),
                    (e),
                    (e),
                    (e),
                    (e),
                    (e),
                    (e),
                ))
            ></p>
        });
        assert_eq!(html, r#"<p class="e e e e e e e e e e e e e e"></p>"#);
    })
    .await;
}

#[tokio::test]
async fn class_builds_outside_a_view() {
    topcoat::view::scope(async {
        // No component or context in scope; the class list is a plain value.
        fn build() -> Class<impl topcoat::view::ClassEntries> {
            class!("btn", "active" if true)
        }

        let classes = build();
        let cx = &Cx::default();
        let html = r(view! { cx => <p class=(classes)></p> });
        assert_eq!(html, r#"<p class="btn active"></p>"#);
    })
    .await;
}
