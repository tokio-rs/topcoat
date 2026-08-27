use topcoat::{
    context::Cx,
    view::{View, ViewExt, view},
};

async fn r(v: impl View) -> String {
    let cx = Cx::default();
    v.single().await.unwrap().render(&cx)
}

#[tokio::test]
async fn empty_view_renders_to_empty_string() {
    let cx = &Cx::default();
    let html = r(view! { cx => }).await;
    assert_eq!(html, "");
}

#[tokio::test]
async fn single_element_renders_with_open_and_close_tags() {
    let cx = &Cx::default();
    let html = r(view! { cx => <p>"hello"</p> }).await;
    assert_eq!(html, "<p>hello</p>");
}

#[tokio::test]
async fn void_elements_render_without_closing_tag() {
    let cx = &Cx::default();
    let html = r(view! {
        cx =>
        <input>
        <br>
        <hr>
    })
    .await;
    assert_eq!(html, "<input><br><hr>");
}

#[tokio::test]
async fn nested_elements_render_in_order() {
    let cx = &Cx::default();
    let html = r(view! {
        cx =>
        <div>
            <span>"a"</span>
            <span>"b"</span>
        </div>
    })
    .await;
    assert_eq!(html, "<div><span>a</span><span>b</span></div>");
}

#[tokio::test]
async fn rust_keyword_element_names_render() {
    let cx = &Cx::default();
    let html = r(view! { cx => <svg><use href="#icon"></use></svg> }).await;
    assert_eq!(html, r##"<svg><use href="#icon"></use></svg>"##);
}

#[tokio::test]
async fn literal_attributes_render_quoted() {
    let cx = &Cx::default();
    let html = r(view! { cx => <a href="/x" class="link">"go"</a> }).await;
    assert_eq!(html, r#"<a href="/x" class="link">go</a>"#);
}

#[tokio::test]
async fn rust_expression_in_child_position_becomes_a_node() {
    let name = "world";
    let cx = &Cx::default();
    let html = r(view! {
        cx =>
        <h1>
            "Hello, "
            (name)
            "!"
        </h1>
    })
    .await;
    assert_eq!(html, "<h1>Hello, world!</h1>");
}

#[tokio::test]
async fn rust_expression_in_attribute_value_becomes_the_value() {
    let url = "/about";
    let cx = &Cx::default();
    let html = r(view! { cx => <a href=(url)>"about"</a> }).await;
    assert_eq!(html, r#"<a href="/about">about</a>"#);
}

#[tokio::test]
async fn dynamic_attribute_name_uses_parenthesized_expression() {
    let attr = "data-state";
    let cx = &Cx::default();
    let html = r(view! { cx => <div (attr)="ready"></div> }).await;
    assert_eq!(html, r#"<div data-state="ready"></div>"#);
}

#[tokio::test]
async fn dynamic_element_name_uses_parenthesized_expression() {
    let tag: String = "section".to_owned();
    let cx = &Cx::default();
    let html = r(view! { cx => <(tag)>"body"</(tag)> }).await;
    assert_eq!(html, "<section>body</section>");
}

#[tokio::test]
async fn child_text_is_html_escaped() {
    let raw = "<script>alert(1)</script>";
    let cx = &Cx::default();
    let html = r(view! { cx => <p>(raw)</p> }).await;
    assert_eq!(html, "<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>");
}

#[tokio::test]
async fn numeric_child_values_render_as_text() {
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
    })
    .await;
    assert_eq!(html, "<span>42 1.5</span>");
}

#[tokio::test]
async fn conditional_attribute_false_omits_attribute() {
    let disabled = false;
    let cx = &Cx::default();
    let html = r(view! { cx => <button disabled=(disabled)>"go"</button> }).await;
    assert_eq!(html, "<button>go</button>");
}

#[tokio::test]
async fn conditional_attribute_true_renders_empty_value() {
    let disabled = true;
    let cx = &Cx::default();
    let html = r(view! { cx => <button disabled=(disabled)>"go"</button> }).await;
    assert_eq!(html, r#"<button disabled="">go</button>"#);
}

#[tokio::test]
async fn conditional_attribute_none_omits_attribute() {
    let title: Option<&str> = None;
    let cx = &Cx::default();
    let html = r(view! { cx => <button title=(title)>"go"</button> }).await;
    assert_eq!(html, "<button>go</button>");
}

#[tokio::test]
async fn conditional_attribute_some_renders_with_inner_value() {
    let title: Option<&str> = Some("hi");
    let cx = &Cx::default();
    let html = r(view! { cx => <button title=(title)>"go"</button> }).await;
    assert_eq!(html, r#"<button title="hi">go</button>"#);
}

#[tokio::test]
async fn literal_attribute_is_always_present_regardless_of_value() {
    let cx = &Cx::default();
    let html = r(view! { cx => <button disabled="false">"go"</button> }).await;
    assert_eq!(html, r#"<button disabled="false">go</button>"#);
}

#[tokio::test]
async fn doctype_renders_as_html_doctype() {
    let cx = &Cx::default();
    let html = r(view! {
        cx =>
        <!DOCTYPE html>
        <html></html>
    })
    .await;
    assert_eq!(html, "<!DOCTYPE html><html></html>");
}
