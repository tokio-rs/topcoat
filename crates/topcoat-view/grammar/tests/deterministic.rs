use quote::quote;
use topcoat_core_grammar::testing::assert_deterministic;
use topcoat_view_grammar::{
    attributes::Attributes,
    class::Class,
    component::Component,
    live::{Emit, Live},
    props::Props,
    view::View,
};

#[test]
fn view() {
    let inputs = [
        r#"<!DOCTYPE html> <p class="a b" id=(id)>"text " (value)</p> <br>"#,
        r"cx => <ul> #[key(item.id)] for item in items { <li>(item.name)</li> } </ul>",
        r#"if ok { <p>"yes"</p> } else if other { "maybe" } else { <p>"no"</p> }"#,
        r#"match state { State::A => <p>"a"</p>, State::B => { <p>"b"</p> <p>"c"</p> } }"#,
        r#"let label = format!("{n}"); <p>(label)</p>"#,
        r"<input :value=$(query.get()) @input=$(|e: Event| query.set(e.target.value))>",
        r"<div disabled=(true) title=(None::<&str>) (attrs)>(slot)</div>",
        r#"panel(title: "Hi", <p>"Child"</p>) panel(title: "Empty")"#,
        r#"<(tag)>"body"</(tag)>"#,
        r#"<(outer)><(inner)>"body"</(inner)></(outer)>"#,
    ];
    for input in inputs {
        assert_deterministic(|| syn::parse_str::<View>(input));
    }
}

#[test]
fn live() {
    assert_deterministic(|| {
        syn::parse_str::<Live>(r#"emit! { "Loading" }?; let x = fetch().await?; emit! { (x) }"#)
    });
    assert_deterministic(|| syn::parse_str::<Live>(r#"cx => emit! { <p>"Ready"</p> }"#));
}

#[test]
fn emit() {
    assert_deterministic(|| syn::parse_str::<Emit>(r#"<p>"Ready " (value)</p>"#));
}

#[test]
fn attributes() {
    assert_deterministic(|| {
        syn::parse_str::<Attributes>(r#"id="x" class=(class) disabled=(true) (rest)"#)
    });
}

#[test]
fn class() {
    assert_deterministic(|| {
        syn::parse_str::<Class>(r#""base", optional, "on" if active else "off""#)
    });
}

#[test]
fn component() {
    assert_deterministic(|| {
        Component::parse(
            quote! {},
            quote! {
                async fn panel(
                    cx: &Cx,
                    title: &str,
                    #[default] subtitle: Option<String>,
                    #[into] label: String,
                    #[default] child: Child<'_>,
                ) -> Result<impl View> {
                    Ok(view! { <section>(title)(child)</section> })
                }
            },
        )
    });
}

#[test]
fn props() {
    assert_deterministic(|| {
        Props::parse(quote! {
            struct PanelProps<'a> {
                title: &'a str,
                #[default]
                subtitle: Option<String>,
                #[default(3)]
                level: u8,
                #[into]
                label: String,
            }
        })
    });
}
