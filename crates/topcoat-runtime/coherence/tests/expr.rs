use serde::Serialize;
use topcoat::runtime::{Expr, Js, Surrogated, expr};
use topcoat_runtime_coherence::{Case, Observe};

#[test]
fn nested_expressions_inline_values_and_preserve_scope() {
    let active = expr!(true);
    let label = expr!(if active { "Selected" } else { "Other" });
    Case::evaluated("nested label", expr!(label.to_owned()))
        .check()
        .unwrap();

    let value = expr!({
        let number = 3.0;
        number + 1.0
    });
    Case::evaluated(
        "nested scopes",
        expr!({
            let number = 10.0;
            value + number + value
        }),
    )
    .check()
    .unwrap();

    let text = Expr::from("\"<>&\n__external0");
    Case::evaluated("captured source escaping", expr!(text.to_owned()))
        .check()
        .unwrap();
}

#[test]
fn captured_javascript_runs_at_each_use_inside_a_handler() {
    let current = expr!(raw!("cx.hydrate(globalThis.current)", false));
    let handler = expr!(|| if current { "selected" } else { "other" });
    let (_, js) = handler.into_evaluated_and_js();
    let source = format!(
        "(() => {{
            globalThis.current = false;
            const handler = {};
            const first = handler().toString();
            globalThis.current = true;
            return cx.hydrate(first + ',' + handler().toString());
        }})()",
        js.to_source(),
    );
    Case::evaluated(
        "handler sees current captured value",
        Expr::evaluate(|| "other,selected", Js::source(source)),
    )
    .check()
    .unwrap();
}

fn check_conversion<T>(value: T)
where
    T: Surrogated + Observe,
    T::Surrogate: Serialize,
{
    let expression = Expr::from(value);
    assert!(expression.is_static());
    Case::evaluated("converted value", expression)
        .check()
        .unwrap();
}

#[test]
fn conversion_preserves_primitive_values() {
    check_conversion(());
    check_conversion(false);
    check_conversion(true);
    check_conversion(-0.0);
    check_conversion(42.5);
    check_conversion("");
    check_conversion("\"<>&\n\u{1f980}");
    check_conversion(String::from("owned"));
}

#[test]
fn conversion_preserves_integer_types_and_precision() {
    macro_rules! integers {
        ($($ty:ty),* $(,)?) => {
            $(
                check_conversion(<$ty>::MIN);
                check_conversion(<$ty>::MAX);
            )*
        };
    }

    integers!(
        u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
    );
}

#[test]
fn conversion_preserves_compound_values() {
    check_conversion(None::<String>);
    check_conversion(Some(Some(String::from("nested"))));
    check_conversion(Ok::<_, String>(u128::MAX));
    check_conversion(Err::<bool, _>(String::from("error")));
    check_conversion(vec![1i64, i64::MAX]);
    check_conversion([true, false]);
}

#[test]
fn conversion_preserves_borrowed_values() {
    let text = String::from("borrowed");
    let numbers = vec![1u128, u128::MAX];
    check_conversion(&text);
    check_conversion(&numbers);
    check_conversion(numbers.as_slice());
}

#[test]
fn conversions_share_the_known_capture_limitations() {
    Case::evaluated("converted tuple", Expr::from((1.5, 2.5))).assert_known("captured_tuple_value");
    Case::evaluated("converted NaN", Expr::from(f64::NAN)).assert_known("captured_nan");
    Case::evaluated("converted infinity", Expr::from(f64::INFINITY))
        .assert_known("captured_infinity");
}
