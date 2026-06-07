use topcoat::{
    context::Cx,
    runtime::Expr,
    view::{View, ViewPart, ViewParts},
};

fn render(part: ViewPart) -> String {
    let mut parts = ViewParts::new();
    parts.push(part);
    View::new(parts).render(&Cx::empty())
}

#[test]
fn raw_macro_emits_raw_js_and_evaluates_rust_expr() {
    let expr = topcoat::runtime::expr! {
        raw!("Math.max(1, 2)", 2.0)
    };

    let (evaluated, js) = expr.into_evaluated_and_js();

    assert_eq!(evaluated, 2.0);
    assert_eq!(render(js), "Math.max(1, 2)");
}

#[test]
fn raw_macro_interpolates_locals_and_desurrogates_them_for_rust() {
    let expr = topcoat::runtime::expr! {{
        let x = 5;
        raw!("${x} + 5", x + 5)
    }};

    let (evaluated, js) = expr.into_evaluated_and_js();

    assert_eq!(evaluated, 10);
    assert_eq!(
        render(js),
        "(() =&gt; { let __local0 = cx.s({&quot;t&quot;:&quot;i32&quot;,&quot;v&quot;:5}); return __local0 + 5; })()"
    );
}

#[test]
fn raw_macro_interpolates_externals_without_shadowing_rust_expr() {
    let x = 5.0;
    let expr = topcoat::runtime::expr! {
        raw!("${x} + 5", x + 5.0)
    };

    let (evaluated, js) = expr.into_evaluated_and_js();

    assert_eq!(evaluated, 10.0);
    assert_eq!(
        render(js),
        "(() => { const [__external0] = [cx.s({&quot;t&quot;:&quot;f64&quot;,&quot;v&quot;:5.0})]; return __external0 + 5; })()"
    );
}

#[test]
fn raw_macro_without_rust_expr_can_typecheck_in_closure_body() {
    let expr: Expr<_> = topcoat::runtime::expr! {
        |_e: topcoat::runtime::Event| raw!("console.log(${_e})")
    };

    let (_, js) = expr.into_evaluated_and_js();

    assert_eq!(render(js), "(__local0) =&gt; console.log(__local0)");
}
