use topcoat::{
    Result,
    context::Cx,
    view::{Component, View, component, pass::Driver, view},
};

/// Drives a component as its own page and returns the final HTML.
async fn render<C>(cx: &Cx, marker: C, props: C::Props) -> Result<String>
where
    C: Component,
{
    let fut = C::render(marker, cx.detach(), props);
    let mut driver = Driver::new(cx.detach(), fut);
    driver.render_blocking().await
}

/// Renders a component marker with `prop = value` arguments.
macro_rules! drive {
    ($cx:expr, $c:ident $(, $prop:ident = $value:expr)* $(,)?) => {
        render($cx, $c::default(), $c::props_builder()$(.$prop($value))*.build())
    };
}

#[component]
async fn greeting(name: &str) -> Result {
    view! {
        <h1>
            "Hello, "
            (name)
            "!"
        </h1>
    }
}

#[component]
async fn greeting_page() -> Result {
    view! { <main>greeting(name: "Ada")</main> }
}

#[tokio::test]
async fn component_with_named_arg_renders_inline() {
    let cx = Cx::default();
    let html = drive!(&cx, greeting_page).await.unwrap();

    assert_eq!(html, "<main><h1>Hello, Ada!</h1></main>");
}

#[component]
async fn badge(label: &str, tone: &str) -> Result {
    view! { <span class=(format!("badge badge-{tone}"))>(label)</span> }
}

#[component]
async fn badge_page() -> Result {
    view! { badge(label: "New", tone: "success") }
}

#[tokio::test]
async fn component_with_multiple_named_args_renders_attributes() {
    let cx = Cx::default();
    let html = drive!(&cx, badge_page).await.unwrap();

    assert_eq!(html, r#"<span class="badge badge-success">New</span>"#);
}

#[component]
async fn panel(title: &str, child: View) -> Result {
    view! {
        <section class="panel">
            <h2>(title)</h2>
            <div class="body">(child?)</div>
        </section>
    }
}

#[component]
async fn panel_page() -> Result {
    view! {
        panel(
            title: "Profile",
            <p>"hello"</p>
            <p>"world"</p>
        )
    }
}

#[tokio::test]
async fn component_with_trailing_child_nodes_collects_them_as_child_view() {
    let cx = Cx::default();
    let html = drive!(&cx, panel_page).await.unwrap();

    assert_eq!(
        html,
        "<section class=\"panel\"><h2>Profile</h2><div class=\"body\"><p>hello</p><p>world</p></div></section>",
    );
}

#[component]
async fn nested_caller(child: View) -> Result {
    view! { panel(title: "Outer", (child?)) }
}

#[component]
async fn nested_caller_page() -> Result {
    view! { nested_caller(<em>"inner"</em>) }
}

#[tokio::test]
async fn component_can_call_other_components_and_forward_child_views() {
    let cx = Cx::default();
    let html = drive!(&cx, nested_caller_page).await.unwrap();

    assert!(html.contains("<h2>Outer</h2>"));
    assert!(html.contains("<em>inner</em>"));
}

#[component]
async fn no_args_component() -> Result {
    view! { <p>"static"</p> }
}

#[component]
async fn no_args_page() -> Result {
    view! { no_args_component() }
}

#[tokio::test]
async fn component_without_args_renders() {
    let cx = Cx::default();
    let html = drive!(&cx, no_args_page).await.unwrap();

    assert_eq!(html, "<p>static</p>");
}

#[component]
async fn uses_cx(cx: &Cx) -> Result {
    let _ = cx;
    view! { <p>"cx component"</p> }
}

#[component]
async fn uses_cx_page() -> Result {
    view! { uses_cx() }
}

#[tokio::test]
async fn component_can_take_cx_param() {
    let cx = Cx::default();
    let html = drive!(&cx, uses_cx_page).await.unwrap();

    assert_eq!(html, "<p>cx component</p>");
}

#[component]
async fn shout(label: impl Into<String> + Send) -> Result {
    let label: String = label.into();
    view! { <b>(label.to_uppercase())</b> }
}

#[component]
async fn shout_page() -> Result {
    view! {
        shout(label: "hi")
        shout(label: String::from("owned"))
    }
}

#[tokio::test]
async fn component_with_impl_trait_param_accepts_any_impl() {
    let cx = Cx::default();
    let html = drive!(&cx, shout_page).await.unwrap();

    assert_eq!(html, "<b>HI</b><b>OWNED</b>");
}

#[component]
async fn item_list(items: impl IntoIterator<Item = u8> + Send) -> Result {
    let items: Vec<u8> = items.into_iter().collect();
    view! {
        <ul>
            for item in items.iter().copied() {
                <li>(item)</li>
            }
        </ul>
    }
}

#[component]
async fn item_list_page() -> Result {
    view! { item_list(items: vec![1, 2, 3]) }
}

#[tokio::test]
async fn component_with_bounded_impl_trait_param_renders() {
    let cx = Cx::default();
    let html = drive!(&cx, item_list_page).await.unwrap();

    assert_eq!(html, "<ul><li>1</li><li>2</li><li>3</li></ul>");
}

#[component]
async fn count<T: Send + Sync>(items: Vec<T>) -> Result {
    view! { <span>(items.len())</span> }
}

#[component]
async fn count_page() -> Result {
    view! { count(items: vec!["a", "b", "c"]) }
}

#[tokio::test]
async fn generic_component_renders() {
    let cx = Cx::default();
    let html = drive!(&cx, count_page).await.unwrap();

    assert_eq!(html, "<span>3</span>");
}

struct TreeNode {
    label: &'static str,
    children: Vec<TreeNode>,
}

#[component(boxed)]
async fn tree(node: &TreeNode) -> Result {
    view! {
        <li>
            (node.label)
            if !node.children.is_empty() {
                <ul>
                    for child in &node.children {
                        tree(key: child.label, node: child)
                    }
                </ul>
            }
        </li>
    }
}

#[component]
async fn tree_page(root: &TreeNode) -> Result {
    view! { <ul>tree(node: root)</ul> }
}

#[tokio::test]
async fn boxed_component_renders_itself_recursively() {
    let cx = Cx::default();
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
    let html = drive!(&cx, tree_page, root = &root).await.unwrap();

    assert_eq!(
        html,
        "<ul><li>root<ul><li>a<ul><li>a1</li></ul></li><li>b</li></ul></li></ul>",
    );
}

// A cycle only needs one boxed component: `odd_steps` stays a plain
// `#[component]` because `even_steps` breaks the cycle for both.
#[component(boxed)]
async fn even_steps(n: u32) -> Result {
    view! {
        <i>(n)</i>
        if n > 0 {
            odd_steps(n: n - 1)
        }
    }
}

#[component]
async fn odd_steps(n: u32) -> Result {
    view! {
        <b>(n)</b>
        if n > 0 {
            even_steps(n: n - 1)
        }
    }
}

#[component]
async fn steps_page() -> Result {
    view! { even_steps(n: 3) }
}

#[tokio::test]
async fn mutually_recursive_components_need_only_one_boxed() {
    let cx = Cx::default();
    let html = drive!(&cx, steps_page).await.unwrap();

    assert_eq!(html, "<i>3</i><b>2</b><i>1</i><b>0</b>");
}
