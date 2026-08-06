use topcoat::{context::Cx, view::view};

fn r(v: topcoat::Result) -> String {
    v.unwrap().render(&Cx::default())
}

#[tokio::test]
async fn empty_view_renders_to_empty_string() {
    topcoat::view::scope(async {
        let html = r(view! {});
        assert_eq!(html, "");
    })
    .await;
}

#[tokio::test]
async fn single_element_renders_with_open_and_close_tags() {
    topcoat::view::scope(async {
        let html = r(view! { <p>"hello"</p> });
        assert_eq!(html, "<p>hello</p>");
    })
    .await;
}

#[tokio::test]
async fn void_elements_render_without_closing_tag() {
    topcoat::view::scope(async {
        let html = r(view! {
            <input>
            <br>
            <hr>
        });
        assert_eq!(html, "<input><br><hr>");
    })
    .await;
}

#[tokio::test]
async fn nested_elements_render_in_order() {
    topcoat::view::scope(async {
        let html = r(view! {
            <div>
                <span>"a"</span>
                <span>"b"</span>
            </div>
        });
        assert_eq!(html, "<div><span>a</span><span>b</span></div>");
    })
    .await;
}

#[tokio::test]
async fn rust_keyword_element_names_render() {
    topcoat::view::scope(async {
        let html = r(view! { <svg><use href="#icon"></use></svg> });
        assert_eq!(html, r##"<svg><use href="#icon"></use></svg>"##);
    })
    .await;
}

#[tokio::test]
async fn literal_attributes_render_quoted() {
    topcoat::view::scope(async {
        let html = r(view! { <a href="/x" class="link">"go"</a> });
        assert_eq!(html, r#"<a href="/x" class="link">go</a>"#);
    })
    .await;
}

#[tokio::test]
async fn rust_expression_in_child_position_becomes_a_node() {
    topcoat::view::scope(async {
        let name = "world";
        let cx = &Cx::default();
        let html = r(view! {
            cx =>
            <h1>
                "Hello, "
                (name)
                "!"
            </h1>
        });
        assert_eq!(html, "<h1>Hello, world!</h1>");
    })
    .await;
}

#[tokio::test]
async fn rust_expression_in_attribute_value_becomes_the_value() {
    topcoat::view::scope(async {
        let url = "/about";
        let cx = &Cx::default();
        let html = r(view! { cx => <a href=(url)>"about"</a> });
        assert_eq!(html, r#"<a href="/about">about</a>"#);
    })
    .await;
}

#[tokio::test]
async fn dynamic_attribute_name_uses_parenthesized_expression() {
    topcoat::view::scope(async {
        let attr = "data-state";
        let cx = &Cx::default();
        let html = r(view! { cx => <div (attr)="ready"></div> });
        assert_eq!(html, r#"<div data-state="ready"></div>"#);
    })
    .await;
}

#[tokio::test]
async fn dynamic_element_name_uses_parenthesized_expression() {
    topcoat::view::scope(async {
        let tag: String = "section".to_owned();
        let cx = &Cx::default();
        let html = r(view! { cx => <(tag)>"body"</(tag)> });
        assert_eq!(html, "<section>body</section>");
    })
    .await;
}

#[tokio::test]
async fn child_text_is_html_escaped() {
    topcoat::view::scope(async {
        let raw = "<script>alert(1)</script>";
        let cx = &Cx::default();
        let html = r(view! { cx => <p>(raw)</p> });
        assert_eq!(html, "<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>");
    })
    .await;
}

#[tokio::test]
async fn numeric_child_values_render_as_text() {
    topcoat::view::scope(async {
        let count: i32 = 42;
        let ratio: f64 = 1.5;
        let cx = &Cx::default();
        let html = r(view! {
            cx =>
            <span>
                (count)
                " "
                (ratio)
            </span>
        });
        assert_eq!(html, "<span>42 1.5</span>");
    })
    .await;
}

#[tokio::test]
async fn conditional_attribute_false_omits_attribute() {
    topcoat::view::scope(async {
        let disabled = false;
        let cx = &Cx::default();
        let html = r(view! { cx => <button disabled=(disabled)>"go"</button> });
        assert_eq!(html, "<button>go</button>");
    })
    .await;
}

#[tokio::test]
async fn conditional_attribute_true_renders_empty_value() {
    topcoat::view::scope(async {
        let disabled = true;
        let cx = &Cx::default();
        let html = r(view! { cx => <button disabled=(disabled)>"go"</button> });
        assert_eq!(html, r#"<button disabled="">go</button>"#);
    })
    .await;
}

#[tokio::test]
async fn conditional_attribute_none_omits_attribute() {
    topcoat::view::scope(async {
        let title: Option<&str> = None;
        let cx = &Cx::default();
        let html = r(view! { cx => <button title=(title)>"go"</button> });
        assert_eq!(html, "<button>go</button>");
    })
    .await;
}

#[tokio::test]
async fn conditional_attribute_some_renders_with_inner_value() {
    topcoat::view::scope(async {
        let title: Option<&str> = Some("hi");
        let cx = &Cx::default();
        let html = r(view! { cx => <button title=(title)>"go"</button> });
        assert_eq!(html, r#"<button title="hi">go</button>"#);
    })
    .await;
}

#[tokio::test]
async fn literal_attribute_is_always_present_regardless_of_value() {
    topcoat::view::scope(async {
        let html = r(view! { <button disabled="false">"go"</button> });
        assert_eq!(html, r#"<button disabled="false">go</button>"#);
    })
    .await;
}

#[tokio::test]
async fn doctype_renders_as_html_doctype() {
    topcoat::view::scope(async {
        let html = r(view! {
            <!DOCTYPE html>
            <html></html>
        });
        assert_eq!(html, "<!DOCTYPE html><html></html>");
    })
    .await;
}
