use topcoat::{
    Result,
    context::Cx,
    view::{Child, View, ViewBuffer, ViewExt, component, view},
};

// `view!` lowers component calls to expressions that reference `__cx` and
// `__buf`. In real code those names are supplied by `#[page]`, `#[layout]`,
// and `#[component]`. These tests stand in for those wrappers by binding them
// by hand and sealing the content into the buffer.
fn empty_cx() -> Cx {
    Cx::default()
}

#[component]
async fn greeting(name: &str) -> Result<impl View> {
    Ok(view! {
        <h1>
            "Hello, "
            (name)
            "!"
        </h1>
    })
}

#[tokio::test]
async fn component_with_named_arg_renders_inline() {
    let cx = empty_cx();
    let __cx = &cx;
    let __buf = &ViewBuffer::new();
    let result = view! { <main>greeting(name: "Ada")</main> };

    assert_eq!(
        result.single().await.unwrap().seal(__buf).render(__cx),
        "<main><h1>Hello, Ada!</h1></main>"
    );
}

#[component]
async fn badge(label: &str, tone: &str) -> Result<impl View> {
    Ok(view! { <span class=(format!("badge badge-{tone}"))>(label)</span> })
}

#[tokio::test]
async fn component_with_multiple_named_args_renders_attributes() {
    let cx = empty_cx();
    let __cx = &cx;
    let __buf = &ViewBuffer::new();
    let result = view! { badge(label: "New", tone: "success") };

    assert_eq!(
        result.single().await.unwrap().seal(__buf).render(__cx),
        r#"<span class="badge badge-success">New</span>"#,
    );
}

#[component]
async fn panel(title: &str, child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <section class="panel">
            <h2>(title)</h2>
            <div class="body">(child)</div>
        </section>
    })
}

#[tokio::test]
async fn component_with_trailing_child_nodes_collects_them_as_child_view() {
    let cx = empty_cx();
    let __cx = &cx;
    let __buf = &ViewBuffer::new();
    let result = view! {
        panel(
            title: "Profile",
            <p>"hello"</p>
            <p>"world"</p>
        )
    };

    assert_eq!(
        result.single().await.unwrap().seal(__buf).render(__cx),
        "<section class=\"panel\"><h2>Profile</h2><div class=\"body\"><p>hello</p><p>world</p></div></section>",
    );
}

#[component]
async fn nested_caller(child: Child<'_>) -> Result<impl View> {
    Ok(view! { panel(title: "Outer", (child)) })
}

#[tokio::test]
async fn component_can_call_other_components_and_forward_child_views() {
    let cx = empty_cx();
    let __cx = &cx;
    let __buf = &ViewBuffer::new();
    let result = view! { nested_caller(<em>"inner"</em>) };
    let html = result.single().await.unwrap().seal(__buf).render(__cx);

    assert!(html.contains("<h2>Outer</h2>"));
    assert!(html.contains("<em>inner</em>"));
}

#[component]
async fn no_args_component() -> Result<impl View> {
    Ok(view! { <p>"static"</p> })
}

#[tokio::test]
async fn component_without_args_renders() {
    let cx = empty_cx();
    let __cx = &cx;
    let __buf = &ViewBuffer::new();
    let result = view! { no_args_component() };

    assert_eq!(
        result.single().await.unwrap().seal(__buf).render(__cx),
        "<p>static</p>"
    );
}

#[component]
async fn uses_cx(cx: &Cx) -> Result<impl View> {
    let _ = cx;
    Ok(view! { <p>"cx component"</p> })
}

#[tokio::test]
async fn component_can_take_cx_param() {
    let cx = empty_cx();
    let __cx = &cx;
    let __buf = &ViewBuffer::new();
    let result = view! { uses_cx() };

    assert_eq!(
        result.single().await.unwrap().seal(__buf).render(__cx),
        "<p>cx component</p>"
    );
}

#[component]
async fn shout(label: impl Into<String> + Send) -> Result<impl View> {
    let label: String = label.into();
    Ok(view! { <b>(label.to_uppercase())</b> })
}

#[tokio::test]
async fn component_with_impl_trait_param_accepts_any_impl() {
    let cx = empty_cx();
    let __cx = &cx;
    let __buf = &ViewBuffer::new();
    let result = view! { shout(label: "hi") };

    assert_eq!(
        result.single().await.unwrap().seal(__buf).render(__cx),
        "<b>HI</b>"
    );

    let result = view! { shout(label: String::from("owned")) };

    assert_eq!(
        result.single().await.unwrap().seal(__buf).render(__cx),
        "<b>OWNED</b>"
    );
}

#[component]
async fn item_list(items: impl IntoIterator<Item = u8> + Send) -> Result<impl View> {
    Ok(view! {
        <ul>
            for item in items {
                <li>(item)</li>
            }
        </ul>
    })
}

#[tokio::test]
async fn component_with_bounded_impl_trait_param_renders() {
    let cx = empty_cx();
    let __cx = &cx;
    let __buf = &ViewBuffer::new();
    let result = view! { item_list(items: vec![1, 2, 3]) };

    assert_eq!(
        result.single().await.unwrap().seal(__buf).render(__cx),
        "<ul><li>1</li><li>2</li><li>3</li></ul>",
    );
}

#[component]
async fn count<T: Send + Sync>(items: Vec<T>) -> Result<impl View> {
    Ok(view! { <span>(items.len())</span> })
}

#[tokio::test]
async fn generic_component_renders() {
    let cx = empty_cx();
    let __cx = &cx;
    let __buf = &ViewBuffer::new();
    let result = view! { count(items: vec!["a", "b", "c"]) };

    assert_eq!(
        result.single().await.unwrap().seal(__buf).render(__cx),
        "<span>3</span>"
    );
}

struct TreeNode {
    label: &'static str,
    children: Vec<TreeNode>,
}

#[component(boxed)]
async fn tree(node: &TreeNode) -> Result<impl View> {
    Ok(view! {
        <li>
            (node.label)
            if !node.children.is_empty() {
                <ul>
                    for child in &node.children {
                        tree(node: child)
                    }
                </ul>
            }
        </li>
    })
}

#[tokio::test]
async fn boxed_component_renders_itself_recursively() {
    let cx = empty_cx();
    let __cx = &cx;
    let __buf = &ViewBuffer::new();
    let root = TreeNode {
        label: "root",
        children: vec![
            TreeNode {
                label: "a",
                children: vec![TreeNode {
                    label: "a1",
                    children: vec![],
                }],
            },
            TreeNode {
                label: "b",
                children: vec![],
            },
        ],
    };
    let result = view! { <ul>tree(node: &root)</ul> };

    assert_eq!(
        result.single().await.unwrap().seal(__buf).render(__cx),
        "<ul><li>root<ul><li>a<ul><li>a1</li></ul></li><li>b</li></ul></li></ul>",
    );
}

// A cycle only needs one boxed component: `odd_steps` stays a plain
// `#[component]` because `even_steps` breaks the cycle for both.
#[component(boxed)]
async fn even_steps(n: u32) -> Result<impl View> {
    Ok(view! {
        <i>(n)</i>
        if n > 0 {
            odd_steps(n: n - 1)
        }
    })
}

#[component]
async fn odd_steps(n: u32) -> Result<impl View> {
    Ok(view! {
        <b>(n)</b>
        if n > 0 {
            even_steps(n: n - 1)
        }
    })
}

#[tokio::test]
async fn mutually_recursive_components_need_only_one_boxed() {
    let cx = empty_cx();
    let __cx = &cx;
    let __buf = &ViewBuffer::new();
    let result = view! { even_steps(n: 3) };

    assert_eq!(
        result.single().await.unwrap().seal(__buf).render(__cx),
        "<i>3</i><b>2</b><i>1</i><b>0</b>",
    );
}
