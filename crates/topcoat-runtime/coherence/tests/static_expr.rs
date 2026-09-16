use serde::Serialize;
use topcoat::runtime::{Expr, Surrogated};
use topcoat_runtime_coherence::{Case, Observe};

fn check<T>(value: T)
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
fn primitive_values() {
    check(());
    check(false);
    check(true);
    check(-0.0);
    check(42.5);
    check("");
    check("\"<>&\n\u{1f980}");
    check(String::from("owned"));
}

#[test]
fn integer_values_preserve_their_type_and_precision() {
    macro_rules! integers {
        ($($ty:ty),* $(,)?) => {
            $(
                check(<$ty>::MIN);
                check(<$ty>::MAX);
            )*
        };
    }

    integers!(
        u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
    );
}

#[test]
fn compound_values() {
    check(None::<String>);
    check(Some(Some(String::from("nested"))));
    check(Ok::<_, String>(u128::MAX));
    check(Err::<bool, _>(String::from("error")));
    check(vec![1i64, i64::MAX]);
    check([true, false]);
}

#[test]
fn borrowed_values() {
    let text = String::from("borrowed");
    let numbers = vec![1u128, u128::MAX];
    check(&text);
    check(&numbers);
    check(numbers.as_slice());
}

#[test]
fn conversions_share_the_known_capture_limitations() {
    Case::evaluated("converted tuple", Expr::from((1.5, 2.5))).assert_known("captured_tuple_value");
    Case::evaluated("converted NaN", Expr::from(f64::NAN)).assert_known("captured_nan");
    Case::evaluated("converted infinity", Expr::from(f64::INFINITY))
        .assert_known("captured_infinity");
}
