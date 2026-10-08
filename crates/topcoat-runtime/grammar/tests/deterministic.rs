use quote::quote;
use topcoat_core_grammar::testing::assert_deterministic;
use topcoat_runtime_grammar::{
    expr::ExprInput, procedure::Procedure, record::Record, shard::Shard,
};

#[test]
fn expr() {
    let inputs = [
        "count.get() + 1",
        r#"if active { "yes" } else { "no" }"#,
        "|e: Event| query.set(e.target.value)",
        "async |_e: Event| { save(query.get()).await }",
        "{ let doubled = value * 2; items.iter().map(|item| item.len()).sum::<usize>() + doubled }",
        r#"raw!("${count}.toFixed(2)", count.get().to_string())"#,
        "todo.read().title.clone()",
    ];
    for input in inputs {
        assert_deterministic(|| syn::parse_str::<ExprInput>(input));
    }
}

#[test]
fn procedure_with_path() {
    assert_deterministic(|| {
        Procedure::parse(
            quote! { "/api/double" },
            quote! {
                async fn double(cx: &Cx, value: f64) -> Result<f64> {
                    Ok(value * 2.0)
                }
            },
        )
    });
}

#[test]
fn procedure_without_path() {
    assert_deterministic(|| {
        Procedure::parse(
            quote! {},
            quote! {
                async fn save(cx: &Cx, enabled: bool, label: String) -> Result<String> {
                    Ok(label)
                }
            },
        )
    });
}

#[test]
fn record() {
    assert_deterministic(|| {
        Record::parse(
            quote! {},
            quote! {
                #[derive(Clone)]
                pub struct Todo {
                    pub title: String,
                    pub done: bool,
                    tags: Vec<String>,
                }
            },
        )
    });
}

#[test]
fn shard_with_path() {
    assert_deterministic(|| {
        Shard::parse(
            quote! { "/search/results" },
            quote! {
                async fn results(query: String) -> Result<impl View> {
                    Ok(view! { <p>(query)</p> })
                }
            },
        )
    });
}

#[test]
fn shard_without_path() {
    assert_deterministic(|| {
        Shard::parse(
            quote! {},
            quote! {
                async fn results(cx: &Cx, query: String, page: Signal<usize>) -> Result<impl View> {
                    Ok(view! { <p>(query)</p> })
                }
            },
        )
    });
}
